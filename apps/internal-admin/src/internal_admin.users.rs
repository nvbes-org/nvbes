use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct UserDetail {
    principal_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    email: String,
    name: String,
    principal_status: String,
    user_status: String,
    email_verified_at: Option<DateTime<Utc>>,
    workspace_count: i64,
    active_workspace_count: i64,
    active_mfa_factor_count: i64,
    active_oauth_consent_count: i64,
    risk_events_24h: i64,
    audit_events_24h: i64,
    latest_risk_at: Option<DateTime<Utc>>,
    latest_audit_at: Option<DateTime<Utc>>,
    primary_workspace_id: Option<Uuid>,
    primary_workspace_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/users/{principalId}", get(user_detail_route))
}

async fn user_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<UserDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_user_detail(&state.db, principal_id).await?))
}

async fn load_user_detail(db: &PgPool, principal_id: Uuid) -> Result<UserDetail, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          p.id, p.tenant_id, t.name AS tenant_name, u.email, u.name,
          p.status::text AS principal_status, u.status::text AS user_status,
          u.email_verified_at, u.created_at, u.updated_at,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.principal_id = p.id
          ) AS workspace_count,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.principal_id = p.id AND wm.status::text = 'active'
          ) AS active_workspace_count,
          (
            SELECT COUNT(*) FROM mfa_factors mf
            WHERE mf.principal_id = p.id AND mf.status::text = 'active'
          ) AS active_mfa_factor_count,
          (
            SELECT COUNT(*) FROM oauth_consents oc
            WHERE oc.principal_id = p.id AND oc.revoked_at IS NULL
              AND (oc.expires_at IS NULL OR oc.expires_at > NOW())
          ) AS active_oauth_consent_count,
          (
            SELECT COUNT(*) FROM risk_events re
            WHERE re.principal_id = p.id AND re.created_at >= NOW() - INTERVAL '24 hours'
          ) AS risk_events_24h,
          (
            SELECT COUNT(*) FROM audit_events ae
            WHERE ae.actor_principal_id = p.id AND ae.created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h,
          (SELECT MAX(re.created_at) FROM risk_events re WHERE re.principal_id = p.id)
            AS latest_risk_at,
          (SELECT MAX(ae.created_at) FROM audit_events ae WHERE ae.actor_principal_id = p.id)
            AS latest_audit_at,
          (
            SELECT wm.workspace_id FROM workspace_memberships wm
            WHERE wm.principal_id = p.id
            ORDER BY wm.updated_at DESC
            LIMIT 1
          ) AS primary_workspace_id,
          (
            SELECT w.name FROM workspace_memberships wm
            JOIN workspaces w ON w.id = wm.workspace_id
            WHERE wm.principal_id = p.id
            ORDER BY wm.updated_at DESC
            LIMIT 1
          ) AS primary_workspace_name
        FROM principals p
        JOIN tenants t ON t.id = p.tenant_id
        JOIN users u ON u.principal_id = p.id
        WHERE p.id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    Ok(UserDetail {
        principal_id: row.get(0),
        tenant_id: row.get(1),
        tenant_name: row.get(2),
        email: row.get(3),
        name: row.get(4),
        principal_status: row.get(5),
        user_status: row.get(6),
        email_verified_at: row.get(7),
        created_at: row.get(8),
        updated_at: row.get(9),
        workspace_count: row.get(10),
        active_workspace_count: row.get(11),
        active_mfa_factor_count: row.get(12),
        active_oauth_consent_count: row.get(13),
        risk_events_24h: row.get(14),
        audit_events_24h: row.get(15),
        latest_risk_at: row.get(16),
        latest_audit_at: row.get(17),
        primary_workspace_id: row.get(18),
        primary_workspace_name: row.get(19),
    })
}
