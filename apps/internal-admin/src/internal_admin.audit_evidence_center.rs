use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct AuditEvidenceSnapshot {
    audit_events_24h: i64,
    actorless_event_count_24h: i64,
    sensitive_action_count_24h: i64,
    missing_hash_count: i64,
    backfilled_hash_count: i64,
    active_signing_key_count: i64,
    deprecated_signing_key_count: i64,
    revoked_signing_key_count: i64,
    recent_audit_events: Vec<AuditEvidenceEvent>,
    actorless_events: Vec<AuditEvidenceEvent>,
    hash_anomalies: Vec<AuditHashAnomaly>,
    signing_keys: Vec<SigningKeySummary>,
}

#[derive(Debug, Serialize)]
struct AuditEvidenceEvent {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    actor_principal_id: Option<Uuid>,
    actor_email: Option<String>,
    action: String,
    target_type: String,
    target_id: Option<Uuid>,
    ip: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AuditHashAnomaly {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    action: String,
    event_hash: String,
    previous_event_hash: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SigningKeySummary {
    kid: String,
    algorithm: String,
    status: String,
    kms_key_id: Option<String>,
    activated_at: DateTime<Utc>,
    deprecated_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/audit-evidence-center", get(audit_evidence_route))
}

async fn audit_evidence_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuditEvidenceSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_audit_evidence(&state.db).await?))
}

async fn load_audit_evidence(db: &PgPool) -> Result<AuditEvidenceSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM audit_events WHERE created_at >= NOW() - INTERVAL '24 hours')
            AS audit_events_24h,
          (
            SELECT COUNT(*) FROM audit_events
            WHERE actor_principal_id IS NULL AND created_at >= NOW() - INTERVAL '24 hours'
          ) AS actorless_event_count_24h,
          (
            SELECT COUNT(*) FROM audit_events
            WHERE created_at >= NOW() - INTERVAL '24 hours'
              AND (
                action ILIKE '%delete%' OR action ILIKE '%revoke%' OR action ILIKE '%suspend%'
                OR action ILIKE '%refund%' OR action ILIKE '%write%' OR action ILIKE '%break%'
              )
          ) AS sensitive_action_count_24h,
          (SELECT COUNT(*) FROM audit_events WHERE event_hash IS NULL OR event_hash = '')
            AS missing_hash_count,
          (SELECT COUNT(*) FROM audit_events WHERE event_hash = 'backfill')
            AS backfilled_hash_count,
          (SELECT COUNT(*) FROM signing_keys WHERE status = 'active') AS active_signing_key_count,
          (SELECT COUNT(*) FROM signing_keys WHERE status = 'deprecated') AS deprecated_signing_key_count,
          (SELECT COUNT(*) FROM signing_keys WHERE status = 'revoked') AS revoked_signing_key_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(AuditEvidenceSnapshot {
        audit_events_24h: metrics.get("audit_events_24h"),
        actorless_event_count_24h: metrics.get("actorless_event_count_24h"),
        sensitive_action_count_24h: metrics.get("sensitive_action_count_24h"),
        missing_hash_count: metrics.get("missing_hash_count"),
        backfilled_hash_count: metrics.get("backfilled_hash_count"),
        active_signing_key_count: metrics.get("active_signing_key_count"),
        deprecated_signing_key_count: metrics.get("deprecated_signing_key_count"),
        revoked_signing_key_count: metrics.get("revoked_signing_key_count"),
        recent_audit_events: load_audit_events(db, false).await?,
        actorless_events: load_audit_events(db, true).await?,
        hash_anomalies: load_hash_anomalies(db).await?,
        signing_keys: load_signing_keys(db).await?,
    })
}

async fn load_audit_events(
    db: &PgPool,
    actorless_only: bool,
) -> Result<Vec<AuditEvidenceEvent>, AppError> {
    let predicate = if actorless_only {
        "WHERE ae.actor_principal_id IS NULL"
    } else {
        ""
    };
    let query = format!(
        r#"
        SELECT ae.id, ae.tenant_id, t.name AS tenant_name, ae.workspace_id,
          ae.actor_principal_id, u.email AS actor_email, ae.action, ae.target_type,
          ae.target_id, ae.ip::text AS ip, ae.created_at
        FROM audit_events ae
        JOIN tenants t ON t.id = ae.tenant_id
        LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
        {predicate}
        ORDER BY ae.created_at DESC
        LIMIT 8
        "#
    );
    let rows = sqlx::query(&query).fetch_all(db).await?;

    Ok(rows
        .into_iter()
        .map(|row| AuditEvidenceEvent {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            actor_principal_id: row.get("actor_principal_id"),
            actor_email: row.get("actor_email"),
            action: row.get("action"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            ip: row.get("ip"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_hash_anomalies(db: &PgPool) -> Result<Vec<AuditHashAnomaly>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ae.id, ae.tenant_id, t.name AS tenant_name, ae.action, ae.event_hash,
          ae.previous_event_hash, ae.created_at
        FROM audit_events ae
        JOIN tenants t ON t.id = ae.tenant_id
        WHERE ae.event_hash IS NULL OR ae.event_hash = '' OR ae.event_hash = 'backfill'
        ORDER BY ae.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AuditHashAnomaly {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            action: row.get("action"),
            event_hash: row.get("event_hash"),
            previous_event_hash: row.get("previous_event_hash"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_signing_keys(db: &PgPool) -> Result<Vec<SigningKeySummary>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT kid, algorithm, status, kms_key_id, activated_at, deprecated_at, revoked_at
        FROM signing_keys
        ORDER BY activated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| SigningKeySummary {
            kid: row.get("kid"),
            algorithm: row.get("algorithm"),
            status: row.get("status"),
            kms_key_id: row.get("kms_key_id"),
            activated_at: row.get("activated_at"),
            deprecated_at: row.get("deprecated_at"),
            revoked_at: row.get("revoked_at"),
        })
        .collect())
}
