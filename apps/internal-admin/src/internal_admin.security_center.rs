use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct SecurityCenterSnapshot {
    risk_events_24h: i64,
    high_risk_events_24h: i64,
    active_users_without_mfa: i64,
    suspended_principal_count: i64,
    revoked_principal_count: i64,
    unverified_user_count: i64,
    active_oauth_consent_count: i64,
    recent_risk_events: Vec<RecentRiskEvent>,
    users_without_mfa: Vec<UserWithoutMfa>,
    active_mfa_factors: Vec<ActiveMfaFactor>,
    active_oauth_consents: Vec<ActiveOauthConsent>,
}

#[derive(Debug, Serialize)]
struct RecentRiskEvent {
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
struct UserWithoutMfa {
    principal_id: Uuid,
    email: String,
    name: String,
    tenant_id: Uuid,
    tenant_name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ActiveMfaFactor {
    id: Uuid,
    principal_id: Uuid,
    email: String,
    tenant_id: Uuid,
    tenant_name: String,
    factor_type: String,
    label: Option<String>,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ActiveOauthConsent {
    id: Uuid,
    principal_id: Uuid,
    email: String,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    client_id: Uuid,
    scopes: Vec<String>,
    granted_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/security-center", get(security_center_route))
        .merge(crate::security_center_actions::router())
}

async fn security_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SecurityCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_security_center(&state.db).await?))
}

async fn load_security_center(db: &PgPool) -> Result<SecurityCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM risk_events
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS risk_events_24h,
          (
            SELECT COUNT(*) FROM risk_events
            WHERE created_at >= NOW() - INTERVAL '24 hours' AND risk_score >= 0.7
          ) AS high_risk_events_24h,
          (
            SELECT COUNT(*) FROM users u
            JOIN principals p ON p.id = u.principal_id
            WHERE u.status::text = 'active'
              AND NOT EXISTS (
                SELECT 1 FROM mfa_factors mf
                WHERE mf.principal_id = u.principal_id AND mf.status::text = 'active'
              )
          ) AS active_users_without_mfa,
          (
            SELECT COUNT(*) FROM principals
            WHERE status::text = 'suspended'
          ) AS suspended_principal_count,
          (
            SELECT COUNT(*) FROM principals
            WHERE status::text = 'revoked'
          ) AS revoked_principal_count,
          (
            SELECT COUNT(*) FROM users
            WHERE email_verified_at IS NULL AND status::text <> 'deleted'
          ) AS unverified_user_count,
          (
            SELECT COUNT(*) FROM oauth_consents
            WHERE revoked_at IS NULL AND (expires_at IS NULL OR expires_at > NOW())
          ) AS active_oauth_consent_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(SecurityCenterSnapshot {
        risk_events_24h: metrics.get("risk_events_24h"),
        high_risk_events_24h: metrics.get("high_risk_events_24h"),
        active_users_without_mfa: metrics.get("active_users_without_mfa"),
        suspended_principal_count: metrics.get("suspended_principal_count"),
        revoked_principal_count: metrics.get("revoked_principal_count"),
        unverified_user_count: metrics.get("unverified_user_count"),
        active_oauth_consent_count: metrics.get("active_oauth_consent_count"),
        recent_risk_events: load_recent_risk_events(db).await?,
        users_without_mfa: load_users_without_mfa(db).await?,
        active_mfa_factors: load_active_mfa_factors(db).await?,
        active_oauth_consents: load_active_oauth_consents(db).await?,
    })
}

async fn load_recent_risk_events(db: &PgPool) -> Result<Vec<RecentRiskEvent>, AppError> {
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
        .map(|row| RecentRiskEvent {
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

async fn load_users_without_mfa(db: &PgPool) -> Result<Vec<UserWithoutMfa>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT u.principal_id, u.email, u.name, p.tenant_id, t.name AS tenant_name, u.created_at
        FROM users u
        JOIN principals p ON p.id = u.principal_id
        JOIN tenants t ON t.id = p.tenant_id
        WHERE u.status::text = 'active'
          AND NOT EXISTS (
            SELECT 1 FROM mfa_factors mf
            WHERE mf.principal_id = u.principal_id AND mf.status::text = 'active'
          )
        ORDER BY u.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UserWithoutMfa {
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            name: row.get("name"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_active_mfa_factors(db: &PgPool) -> Result<Vec<ActiveMfaFactor>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT mf.id, mf.principal_id, u.email, p.tenant_id, t.name AS tenant_name,
          mf.factor_type::text, mf.label, mf.last_used_at, mf.created_at
        FROM mfa_factors mf
        JOIN principals p ON p.id = mf.principal_id
        JOIN tenants t ON t.id = p.tenant_id
        JOIN users u ON u.principal_id = mf.principal_id
        WHERE mf.status::text = 'active'
        ORDER BY COALESCE(mf.last_used_at, mf.created_at) DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ActiveMfaFactor {
            id: row.get("id"),
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            factor_type: row.get("factor_type"),
            label: row.get("label"),
            last_used_at: row.get("last_used_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_active_oauth_consents(db: &PgPool) -> Result<Vec<ActiveOauthConsent>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT oc.id, oc.principal_id, u.email, oc.tenant_id, t.name AS tenant_name,
          oc.workspace_id, w.name AS workspace_name, oc.client_id, oc.scope,
          oc.granted_at, oc.expires_at
        FROM oauth_consents oc
        JOIN tenants t ON t.id = oc.tenant_id
        JOIN users u ON u.principal_id = oc.principal_id
        LEFT JOIN workspaces w ON w.id = oc.workspace_id
        WHERE oc.revoked_at IS NULL AND (oc.expires_at IS NULL OR oc.expires_at > NOW())
        ORDER BY oc.granted_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ActiveOauthConsent {
            id: row.get("id"),
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            client_id: row.get("client_id"),
            scopes: row.get("scope"),
            granted_at: row.get("granted_at"),
            expires_at: row.get("expires_at"),
        })
        .collect())
}
