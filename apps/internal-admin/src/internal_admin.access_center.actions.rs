use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_confirmation, require_idempotency_key, require_permission,
};
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct AccessActionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct AccessActionResult {
    workspace_id: Uuid,
    tenant_id: Uuid,
    principal_id: Uuid,
    previous_status: String,
    next_status: &'static str,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/access-center/workspace-memberships/{workspaceId}/{principalId}/suspend",
        post(suspend_workspace_membership_route),
    )
}

async fn suspend_workspace_membership_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, principal_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AccessActionRequest>,
) -> Result<Json<AccessActionResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::AccessMutate)?;
    require_confirmation(&request.confirm_code, "SUSPEND ACCESS")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        suspend_workspace_membership(&state.db, actor_id, workspace_id, principal_id, request)
            .await?,
    ))
}

async fn suspend_workspace_membership(
    db: &PgPool,
    actor_id: Uuid,
    workspace_id: Uuid,
    principal_id: Uuid,
    request: AccessActionRequest,
) -> Result<AccessActionResult, AppError> {
    validate_access_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT w.tenant_id, wm.role::text AS role, wm.status::text AS status
        FROM workspace_memberships wm
        JOIN workspaces w ON w.id = wm.workspace_id
        WHERE wm.workspace_id = $1 AND wm.principal_id = $2
        FOR UPDATE OF wm
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let role: String = row.get("role");
    let previous_status: String = row.get("status");
    if previous_status != "active" {
        return Err(AppError::bad_request(
            "workspace_membership_not_active",
            "Only active workspace memberships can be suspended.",
        ));
    }
    if role == "owner" {
        ensure_not_last_owner(tx.as_mut(), workspace_id, principal_id).await?;
    }

    sqlx::query(
        "UPDATE workspace_memberships SET status = 'suspended', updated_at = NOW() WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(tx.as_mut())
    .await?;

    insert_access_audit(
        tx.as_mut(),
        tenant_id,
        workspace_id,
        actor_id,
        principal_id,
        role,
        &previous_status,
        &request.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(AccessActionResult {
        workspace_id,
        tenant_id,
        principal_id,
        previous_status,
        next_status: "suspended",
        audit_action: "internal_admin.access.workspace_membership.suspended",
    })
}

async fn ensure_not_last_owner(
    executor: &mut sqlx::PgConnection,
    workspace_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    let remaining_owner_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM workspace_memberships
        WHERE workspace_id = $1
          AND principal_id <> $2
          AND role::text = 'owner'
          AND status::text = 'active'
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_one(executor)
    .await?;
    if remaining_owner_count == 0 {
        return Err(AppError::bad_request(
            "last_workspace_owner",
            "Cannot suspend the last active owner of a workspace.",
        ));
    }
    Ok(())
}

async fn insert_access_audit(
    executor: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
    principal_id: Uuid,
    role: String,
    previous_status: &str,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'internal_admin.access.workspace_membership.suspended',
           'workspace_membership', $4,
           jsonb_build_object(
             'reason', $5,
             'role', $6,
             'previous_status', $7,
             'next_status', 'suspended',
             'workspace_id', $2::text
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(principal_id)
    .bind(reason.trim())
    .bind(role)
    .bind(previous_status)
    .execute(executor)
    .await?;
    Ok(())
}

fn validate_access_action_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Access actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_action_reason_must_be_detailed() {
        assert!(validate_access_action_reason("short").is_err());
        assert!(validate_access_action_reason("ticket IAM-123 approved").is_ok());
    }
}
