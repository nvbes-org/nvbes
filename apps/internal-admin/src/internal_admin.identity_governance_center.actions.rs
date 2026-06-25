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
    BackofficePermission, require_operator_permission_headers, require_operator_role_grant,
    require_strong_confirmation,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct GovernanceActionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct GovernanceActionResult {
    object_id: Uuid,
    tenant_id: Uuid,
    principal_id: Uuid,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/identity-governance-center/break-glass/{tenantId}/{principalId}/revoke",
            post(revoke_break_glass_route),
        )
        .route(
            "/admin/identity-governance-center/recovery-requests/{requestId}/cancel",
            post(cancel_recovery_request_route),
        )
}

async fn revoke_break_glass_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, principal_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<GovernanceActionRequest>,
) -> Result<Json<GovernanceActionResult>, AppError> {
    require_operator_permission_headers(&headers, BackofficePermission::GovernanceMutate)?;
    require_strong_confirmation(&request.confirm_code, "REVOKE BREAK GLASS", principal_id)?;
    require_dual_control(&headers)?;
    require_operator_role_grant(&state.db, &headers).await?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        revoke_break_glass(&state.db, actor_id, tenant_id, principal_id, request).await?,
    ))
}

async fn cancel_recovery_request_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    Json(request): Json<GovernanceActionRequest>,
) -> Result<Json<GovernanceActionResult>, AppError> {
    require_operator_permission_headers(&headers, BackofficePermission::GovernanceMutate)?;
    require_strong_confirmation(&request.confirm_code, "CANCEL RECOVERY", request_id)?;
    require_dual_control(&headers)?;
    require_operator_role_grant(&state.db, &headers).await?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        cancel_recovery_request(&state.db, actor_id, request_id, request).await?,
    ))
}

async fn revoke_break_glass(
    db: &PgPool,
    actor_id: Uuid,
    tenant_id: Uuid,
    principal_id: Uuid,
    request: GovernanceActionRequest,
) -> Result<GovernanceActionResult, AppError> {
    validate_governance_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let revoked_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT revoked_at FROM tenant_break_glass_accounts WHERE tenant_id = $1 AND principal_id = $2 FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    if revoked_at.is_some() {
        return Err(AppError::bad_request(
            "break_glass_already_revoked",
            "Break-glass account is already revoked.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE tenant_break_glass_accounts
        SET revoked_at = NOW(), revoked_by_principal_id = $3, revoked_reason = $4, updated_at = NOW()
        WHERE tenant_id = $1 AND principal_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(actor_id)
    .bind(request.reason.trim())
    .execute(tx.as_mut())
    .await?;
    insert_governance_audit(
        tx.as_mut(),
        tenant_id,
        actor_id,
        "internal_admin.identity_governance.break_glass.revoked",
        "break_glass_account",
        principal_id,
        principal_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(GovernanceActionResult {
        object_id: principal_id,
        tenant_id,
        principal_id,
        audit_action: "internal_admin.identity_governance.break_glass.revoked",
    })
}

async fn cancel_recovery_request(
    db: &PgPool,
    actor_id: Uuid,
    request_id: Uuid,
    request: GovernanceActionRequest,
) -> Result<GovernanceActionResult, AppError> {
    validate_governance_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "SELECT tenant_id, principal_id, status FROM enterprise_password_recovery_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(request_id)
    .fetch_one(tx.as_mut())
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let principal_id: Uuid = row.get("principal_id");
    let status: String = row.get("status");
    if status != "pending" {
        return Err(AppError::bad_request(
            "recovery_request_not_pending",
            "Only pending recovery requests can be cancelled.",
        ));
    }

    sqlx::query(
        "UPDATE enterprise_password_recovery_requests SET status = 'cancelled', updated_at = NOW() WHERE id = $1",
    )
    .bind(request_id)
    .execute(tx.as_mut())
    .await?;
    insert_governance_audit(
        tx.as_mut(),
        tenant_id,
        actor_id,
        "internal_admin.identity_governance.recovery_request.cancelled",
        "recovery_request",
        request_id,
        principal_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;

    Ok(GovernanceActionResult {
        object_id: request_id,
        tenant_id,
        principal_id,
        audit_action: "internal_admin.identity_governance.recovery_request.cancelled",
    })
}

async fn insert_governance_audit(
    executor: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
    target_type: &str,
    target_id: Uuid,
    principal_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object('reason', $6, 'principal_id', $7::text),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason.trim())
    .bind(principal_id)
    .execute(executor)
    .await?;
    Ok(())
}

fn validate_governance_action_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Identity governance actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_action_reason_must_be_detailed() {
        assert!(validate_governance_action_reason("short").is_err());
        assert!(validate_governance_action_reason("ticket GOV-456 approved").is_ok());
    }
}
