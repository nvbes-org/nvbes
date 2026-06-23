use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct RiskDecisionSnapshot {
    identity_risk_event_count_24h: i64,
    high_identity_risk_event_count_24h: i64,
    billing_risk_signal_count_24h: i64,
    high_billing_risk_score_count: i64,
    active_access_policy_count: i64,
    access_policy_count_24h: i64,
    recent_identity_risks: Vec<IdentityRiskDecision>,
    billing_risk_scores: Vec<BillingRiskScore>,
    billing_risk_signals: Vec<BillingRiskSignal>,
    active_access_policies: Vec<AccessPolicySnapshot>,
}

#[derive(Debug, Serialize)]
struct IdentityRiskDecision {
    id: Uuid,
    principal_id: Uuid,
    email: Option<String>,
    tenant_id: Uuid,
    tenant_name: String,
    event_type: String,
    risk_score: f64,
    decision: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct BillingRiskScore {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    score: f64,
    decision: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct BillingRiskSignal {
    id: Uuid,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
    signal_type: String,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AccessPolicySnapshot {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    policy_state: String,
    reason: String,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/risk-decision-center",
        get(risk_decision_center_route),
    )
}

async fn risk_decision_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RiskDecisionSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_risk_decision(&state.db).await?))
}

async fn load_risk_decision(db: &PgPool) -> Result<RiskDecisionSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM risk_events WHERE created_at >= NOW() - INTERVAL '24 hours')
            AS identity_risk_event_count_24h,
          (
            SELECT COUNT(*) FROM risk_events
            WHERE created_at >= NOW() - INTERVAL '24 hours' AND risk_score >= 0.7
          ) AS high_identity_risk_event_count_24h,
          (
            SELECT COUNT(*) FROM billing_risk_signals
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS billing_risk_signal_count_24h,
          (SELECT COUNT(*) FROM billing_risk_scores WHERE score >= 70)
            AS high_billing_risk_score_count,
          (
            SELECT COUNT(*) FROM billing_access_policy_snapshots
            WHERE effective_from <= NOW() AND (effective_to IS NULL OR effective_to > NOW())
          ) AS active_access_policy_count,
          (
            SELECT COUNT(*) FROM billing_access_policy_snapshots
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS access_policy_count_24h
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(RiskDecisionSnapshot {
        identity_risk_event_count_24h: metrics.get("identity_risk_event_count_24h"),
        high_identity_risk_event_count_24h: metrics.get("high_identity_risk_event_count_24h"),
        billing_risk_signal_count_24h: metrics.get("billing_risk_signal_count_24h"),
        high_billing_risk_score_count: metrics.get("high_billing_risk_score_count"),
        active_access_policy_count: metrics.get("active_access_policy_count"),
        access_policy_count_24h: metrics.get("access_policy_count_24h"),
        recent_identity_risks: load_identity_risks(db).await?,
        billing_risk_scores: load_billing_risk_scores(db).await?,
        billing_risk_signals: load_billing_risk_signals(db).await?,
        active_access_policies: load_access_policies(db).await?,
    })
}

async fn load_identity_risks(db: &PgPool) -> Result<Vec<IdentityRiskDecision>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT re.id, re.principal_id, u.email, p.tenant_id, t.name AS tenant_name,
          re.event_type, re.risk_score, re.decision, re.created_at
        FROM risk_events re
        JOIN principals p ON p.id = re.principal_id
        JOIN tenants t ON t.id = p.tenant_id
        LEFT JOIN users u ON u.principal_id = re.principal_id
        ORDER BY re.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| IdentityRiskDecision {
            id: row.get("id"),
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            event_type: row.get("event_type"),
            risk_score: row.get("risk_score"),
            decision: row.get("decision"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_billing_risk_scores(db: &PgPool) -> Result<Vec<BillingRiskScore>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT brs.id, brs.tenant_id, t.name AS tenant_name, brs.score::float8 AS score,
          brs.decision, brs.created_at
        FROM billing_risk_scores brs
        JOIN tenants t ON t.id = brs.tenant_id
        ORDER BY brs.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BillingRiskScore {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            score: row.get("score"),
            decision: row.get("decision"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_billing_risk_signals(db: &PgPool) -> Result<Vec<BillingRiskSignal>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT brs.id, brs.tenant_id, t.name AS tenant_name, brs.signal_type, brs.occurred_at
        FROM billing_risk_signals brs
        LEFT JOIN tenants t ON t.id = brs.tenant_id
        ORDER BY brs.occurred_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BillingRiskSignal {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            signal_type: row.get("signal_type"),
            occurred_at: row.get("occurred_at"),
        })
        .collect())
}

async fn load_access_policies(db: &PgPool) -> Result<Vec<AccessPolicySnapshot>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT aps.id, aps.tenant_id, t.name AS tenant_name, aps.workspace_id,
          w.name AS workspace_name, aps.policy_state, aps.reason,
          aps.effective_from, aps.effective_to
        FROM billing_access_policy_snapshots aps
        JOIN tenants t ON t.id = aps.tenant_id
        LEFT JOIN workspaces w ON w.id = aps.workspace_id
        WHERE aps.effective_from <= NOW() AND (aps.effective_to IS NULL OR aps.effective_to > NOW())
        ORDER BY aps.effective_from DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AccessPolicySnapshot {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            policy_state: row.get("policy_state"),
            reason: row.get("reason"),
            effective_from: row.get("effective_from"),
            effective_to: row.get("effective_to"),
        })
        .collect())
}
