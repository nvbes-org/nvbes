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
struct WorkspaceDetail {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    name: String,
    status: String,
    workspace_type: String,
    plan_code: String,
    trial_ends_at: Option<DateTime<Utc>>,
    member_count: i64,
    owner_count: i64,
    active_member_count: i64,
    service_account_count: i64,
    audit_events_24h: i64,
    open_invoice_count: i64,
    active_subscription_count: i64,
    latest_audit_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceLifecycleRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct WorkspaceLifecycleResult {
    workspace_id: Uuid,
    tenant_id: Uuid,
    previous_status: String,
    next_status: String,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/workspaces/{workspaceId}",
            get(workspace_detail_route),
        )
        .route(
            "/admin/workspaces/{workspaceId}/suspend",
            post(suspend_workspace_route),
        )
        .route(
            "/admin/workspaces/{workspaceId}/reactivate",
            post(reactivate_workspace_route),
        )
}

async fn workspace_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_workspace_detail(&state.db, workspace_id).await?))
}

async fn suspend_workspace_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<WorkspaceLifecycleRequest>,
) -> Result<Json<WorkspaceLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::WorkspaceLifecycle)?;
    require_operator_role_grant(&state.db, &headers).await?;
    require_strong_confirmation(&request.confirm_code, "SUSPEND WORKSPACE", workspace_id)?;
    require_dual_control(&headers)?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_workspace_status(
            &state.db,
            actor_id,
            workspace_id,
            "suspended",
            "internal_admin.workspace.suspend",
            request,
        )
        .await?,
    ))
}

async fn reactivate_workspace_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<WorkspaceLifecycleRequest>,
) -> Result<Json<WorkspaceLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::WorkspaceLifecycle)?;
    require_operator_role_grant(&state.db, &headers).await?;
    require_strong_confirmation(&request.confirm_code, "REACTIVATE WORKSPACE", workspace_id)?;
    require_dual_control(&headers)?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_workspace_status(
            &state.db,
            actor_id,
            workspace_id,
            "active",
            "internal_admin.workspace.reactivate",
            request,
        )
        .await?,
    ))
}

async fn load_workspace_detail(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<WorkspaceDetail, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          w.id, w.tenant_id, t.name AS tenant_name, w.name,
          w.status::text, w.workspace_type::text, w.plan_code, w.trial_ends_at,
          w.created_at, w.updated_at,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.workspace_id = w.id
          ) AS member_count,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.workspace_id = w.id AND wm.role::text = 'owner'
          ) AS owner_count,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.workspace_id = w.id AND wm.status::text = 'active'
          ) AS active_member_count,
          (
            SELECT COUNT(*) FROM service_accounts sa
            WHERE sa.workspace_id = w.id
          ) AS service_account_count,
          (
            SELECT COUNT(*) FROM audit_events ae
            WHERE ae.workspace_id = w.id AND ae.created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h,
          (
            SELECT COUNT(DISTINCT bi.id)
            FROM billing_invoices bi
            LEFT JOIN billing_accounts ba ON ba.id = bi.billing_account_id
            LEFT JOIN billing_subscriptions bs ON bs.id = bi.subscription_id
            WHERE (ba.workspace_id = w.id OR bs.workspace_id = w.id)
              AND bi.status::text IN ('issued', 'pro_forma')
          ) AS open_invoice_count,
          (
            SELECT COUNT(*) FROM billing_subscriptions bs
            WHERE bs.workspace_id = w.id AND bs.status IN ('active', 'trialing')
          ) AS active_subscription_count,
          (
            SELECT MAX(ae.created_at) FROM audit_events ae
            WHERE ae.workspace_id = w.id
          ) AS latest_audit_at
        FROM workspaces w
        JOIN tenants t ON t.id = w.tenant_id
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(db)
    .await?;

    Ok(WorkspaceDetail {
        id: row.get(0),
        tenant_id: row.get(1),
        tenant_name: row.get(2),
        name: row.get(3),
        status: row.get(4),
        workspace_type: row.get(5),
        plan_code: row.get(6),
        trial_ends_at: row.get(7),
        created_at: row.get(8),
        updated_at: row.get(9),
        member_count: row.get(10),
        owner_count: row.get(11),
        active_member_count: row.get(12),
        service_account_count: row.get(13),
        audit_events_24h: row.get(14),
        open_invoice_count: row.get(15),
        active_subscription_count: row.get(16),
        latest_audit_at: row.get(17),
    })
}

async fn change_workspace_status(
    db: &PgPool,
    actor_id: Uuid,
    workspace_id: Uuid,
    next_status: &'static str,
    audit_action: &'static str,
    request: WorkspaceLifecycleRequest,
) -> Result<WorkspaceLifecycleResult, AppError> {
    validate_lifecycle_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row =
        sqlx::query("SELECT tenant_id, status::text FROM workspaces WHERE id = $1 FOR UPDATE")
            .bind(workspace_id)
            .fetch_one(tx.as_mut())
            .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let previous_status: String = row.get("status");

    validate_workspace_status_transition(&previous_status, next_status)?;

    sqlx::query(
        "UPDATE workspaces SET status = $2::workspace_status, updated_at = NOW() WHERE id = $1",
    )
    .bind(workspace_id)
    .bind(next_status)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, 'workspace', $2,
           jsonb_build_object('reason', $5, 'previous_status', $6, 'next_status', $7),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(audit_action)
    .bind(request.reason.trim())
    .bind(&previous_status)
    .bind(next_status)
    .execute(tx.as_mut())
    .await?;

    tx.commit().await?;

    Ok(WorkspaceLifecycleResult {
        workspace_id,
        tenant_id,
        previous_status,
        next_status: next_status.to_string(),
        audit_action,
    })
}

fn validate_lifecycle_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Workspace lifecycle actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

fn validate_workspace_status_transition(
    previous_status: &str,
    next_status: &'static str,
) -> Result<(), AppError> {
    if previous_status == "deleted" {
        return Err(AppError::bad_request(
            "workspace_deleted",
            "Deleted workspaces cannot be mutated from the back-office.",
        ));
    }
    if previous_status == next_status {
        return Err(AppError::bad_request(
            "workspace_status_unchanged",
            "Workspace is already in the requested status.",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "internal_admin.workspaces.tests.rs"]
mod tests;
