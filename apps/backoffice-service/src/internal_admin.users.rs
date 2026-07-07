use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_idempotency_key, require_operator_role_grant, require_permission,
    require_strong_confirmation,
};
use crate::backoffice_dual_control::require_dual_control;
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

#[derive(Debug, Deserialize)]
struct UserLifecycleRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct UserLifecycleResult {
    principal_id: Uuid,
    tenant_id: Uuid,
    previous_principal_status: String,
    previous_user_status: String,
    next_principal_status: String,
    next_user_status: String,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users/{principalId}", get(user_detail_route))
        .route(
            "/admin/users/{principalId}/suspend",
            post(suspend_user_route),
        )
        .route(
            "/admin/users/{principalId}/reactivate",
            post(reactivate_user_route),
        )
}

async fn user_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<UserDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_user_detail(&state.db, principal_id).await?))
}

async fn suspend_user_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
    Json(request): Json<UserLifecycleRequest>,
) -> Result<Json<UserLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::UserLifecycle)?;
    require_operator_role_grant(&state.db, &headers).await?;
    require_strong_confirmation(&request.confirm_code, "SUSPEND USER", principal_id)?;
    require_dual_control(&headers)?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_user_status(
            &state.db,
            actor_id,
            principal_id,
            UserLifecycleTarget {
                principal_status: "suspended",
                user_status: "suspended",
                audit_action: "internal_admin.user.suspend",
            },
            request,
        )
        .await?,
    ))
}

async fn reactivate_user_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
    Json(request): Json<UserLifecycleRequest>,
) -> Result<Json<UserLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::UserLifecycle)?;
    require_operator_role_grant(&state.db, &headers).await?;
    require_strong_confirmation(&request.confirm_code, "REACTIVATE USER", principal_id)?;
    require_dual_control(&headers)?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_user_status(
            &state.db,
            actor_id,
            principal_id,
            UserLifecycleTarget {
                principal_status: "active",
                user_status: "active",
                audit_action: "internal_admin.user.reactivate",
            },
            request,
        )
        .await?,
    ))
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

#[derive(Debug, Clone, Copy)]
struct UserLifecycleTarget {
    principal_status: &'static str,
    user_status: &'static str,
    audit_action: &'static str,
}

async fn change_user_status(
    db: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
    target: UserLifecycleTarget,
    request: UserLifecycleRequest,
) -> Result<UserLifecycleResult, AppError> {
    validate_lifecycle_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT p.tenant_id, p.status::text AS principal_status, u.status::text AS user_status
        FROM principals p
        JOIN users u ON u.principal_id = p.id
        WHERE p.id = $1
        FOR UPDATE OF p, u
        "#,
    )
    .bind(principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let previous_principal_status: String = row.get("principal_status");
    let previous_user_status: String = row.get("user_status");

    validate_user_status_transition(
        &previous_principal_status,
        &previous_user_status,
        target.principal_status,
        target.user_status,
    )?;

    sqlx::query(
        "UPDATE principals SET status = $2::principal_status, updated_at = NOW() WHERE id = $1",
    )
    .bind(principal_id)
    .bind(target.principal_status)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        "UPDATE users SET status = $2::user_status, updated_at = NOW() WHERE principal_id = $1",
    )
    .bind(principal_id)
    .bind(target.user_status)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'principal', $4,
           jsonb_build_object(
             'reason', $5,
             'previous_principal_status', $6,
             'previous_user_status', $7,
             'next_principal_status', $8,
             'next_user_status', $9
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(target.audit_action)
    .bind(principal_id)
    .bind(request.reason.trim())
    .bind(&previous_principal_status)
    .bind(&previous_user_status)
    .bind(target.principal_status)
    .bind(target.user_status)
    .execute(tx.as_mut())
    .await?;

    tx.commit().await?;

    Ok(UserLifecycleResult {
        principal_id,
        tenant_id,
        previous_principal_status,
        previous_user_status,
        next_principal_status: target.principal_status.to_string(),
        next_user_status: target.user_status.to_string(),
        audit_action: target.audit_action,
    })
}

fn validate_lifecycle_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "User lifecycle actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

fn validate_user_status_transition(
    previous_principal_status: &str,
    previous_user_status: &str,
    next_principal_status: &'static str,
    next_user_status: &'static str,
) -> Result<(), AppError> {
    if matches!(previous_principal_status, "deleted" | "revoked")
        || previous_user_status == "deleted"
    {
        return Err(AppError::bad_request(
            "user_lifecycle_terminal",
            "Deleted or revoked users cannot be mutated from the back-office.",
        ));
    }
    if previous_principal_status == next_principal_status
        && previous_user_status == next_user_status
    {
        return Err(AppError::bad_request(
            "user_status_unchanged",
            "User is already in the requested status.",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "internal_admin.users.tests.rs"]
mod tests;
