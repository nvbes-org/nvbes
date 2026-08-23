use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_idempotency_key, require_operator_role_grant, require_permission,
    require_strong_confirmation,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;
use crate::identity_governance_operator_grant_validation::{
    validate_operator_grant_reason, validate_operator_role,
};

#[derive(Debug, Deserialize)]
struct OperatorGrantRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct OperatorGrantActionResult {
    object_id: Uuid,
    principal_id: Uuid,
    role: String,
    previous_status: Option<String>,
    next_status: String,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/identity-governance-center/operator-grants/{principalId}/{role}/grant",
            post(grant_operator_role_route),
        )
        .route(
            "/admin/identity-governance-center/operator-grants/{principalId}/{role}/revoke",
            post(revoke_operator_role_route),
        )
}

async fn grant_operator_role_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((principal_id, role)): Path<(Uuid, String)>,
    Json(request): Json<OperatorGrantRequest>,
) -> Result<Json<OperatorGrantActionResult>, AppError> {
    require_operator_grant_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "GRANT OPERATOR",
        principal_id,
        &role,
    )
    .await?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        grant_operator_role(&state.db, actor_id, principal_id, role, request).await?,
    ))
}

async fn revoke_operator_role_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((principal_id, role)): Path<(Uuid, String)>,
    Json(request): Json<OperatorGrantRequest>,
) -> Result<Json<OperatorGrantActionResult>, AppError> {
    require_operator_grant_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REVOKE OPERATOR",
        principal_id,
        &role,
    )
    .await?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        revoke_operator_role(&state.db, actor_id, principal_id, role, request).await?,
    ))
}

async fn require_operator_grant_mutation(
    db: &PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    action: &str,
    principal_id: Uuid,
    role: &str,
) -> Result<(), AppError> {
    let normalized_role = validate_operator_role(role)?;
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::GovernanceMutate)?;
    require_operator_role_grant(db, headers).await?;
    require_strong_confirmation(
        confirm_code,
        &format!("{action} {}", normalized_role.to_ascii_uppercase()),
        principal_id,
    )?;
    require_dual_control(headers)
}

async fn grant_operator_role(
    db: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
    role: String,
    request: OperatorGrantRequest,
) -> Result<OperatorGrantActionResult, AppError> {
    let role = validate_operator_role(&role)?.to_string();
    validate_operator_grant_reason(&request.reason)?;
    require_operator_factor_policy(db, principal_id).await?;
    let tenant_id = principal_tenant_id(db, principal_id).await?;
    let previous_status = current_grant_status(db, principal_id, &role).await?;
    if previous_status.as_deref() == Some("active") {
        return Err(AppError::bad_request(
            "operator_grant_already_active",
            "Operator grant is already active.",
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO internal_admin_operator_grants (
          principal_id, role, status, granted_by_principal_id, granted_at, revoked_at, reason
        ) VALUES ($1, $2, 'active', $3, NOW(), NULL, $4)
        ON CONFLICT (principal_id, role) DO UPDATE
        SET status = 'active',
            granted_by_principal_id = EXCLUDED.granted_by_principal_id,
            granted_at = NOW(),
            revoked_at = NULL,
            reason = EXCLUDED.reason
        "#,
    )
    .bind(principal_id)
    .bind(&role)
    .bind(actor_id)
    .bind(request.reason.trim())
    .execute(db)
    .await?;
    insert_operator_grant_audit(
        db,
        tenant_id,
        actor_id,
        principal_id,
        &role,
        previous_status.as_deref(),
        "active",
        "internal_admin.identity_governance.operator_grant.granted",
        &request.reason,
    )
    .await?;

    Ok(OperatorGrantActionResult {
        object_id: principal_id,
        principal_id,
        role,
        previous_status,
        next_status: "active".to_string(),
        audit_action: "internal_admin.identity_governance.operator_grant.granted",
    })
}

async fn require_operator_factor_policy(db: &PgPool, principal_id: Uuid) -> Result<(), AppError> {
    let (factor_count, has_phishing_resistant_factor): (i64, bool) = sqlx::query_as(
        "SELECT
             COUNT(*) FILTER (WHERE factor_type <> 'email'),
             COALESCE(BOOL_OR(factor_type = 'webauthn'), FALSE)
         FROM mfa_factors
         WHERE principal_id = $1 AND status = 'active'",
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    if factor_count < 2 || !has_phishing_resistant_factor {
        return Err(AppError::forbidden(
            "operator_factor_policy_required",
            "Back-office operators must register at least two active factors, including a passkey or hardware security key.",
        ));
    }
    Ok(())
}

async fn revoke_operator_role(
    db: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
    role: String,
    request: OperatorGrantRequest,
) -> Result<OperatorGrantActionResult, AppError> {
    let role = validate_operator_role(&role)?.to_string();
    validate_operator_grant_reason(&request.reason)?;
    let tenant_id = principal_tenant_id(db, principal_id).await?;
    let previous_status = current_grant_status(db, principal_id, &role).await?;
    if previous_status.as_deref() != Some("active") {
        return Err(AppError::bad_request(
            "operator_grant_not_active",
            "Only active operator grants can be revoked.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE internal_admin_operator_grants
        SET status = 'revoked', revoked_at = NOW(), reason = $3
        WHERE principal_id = $1 AND role = $2
        "#,
    )
    .bind(principal_id)
    .bind(&role)
    .bind(request.reason.trim())
    .execute(db)
    .await?;
    insert_operator_grant_audit(
        db,
        tenant_id,
        actor_id,
        principal_id,
        &role,
        previous_status.as_deref(),
        "revoked",
        "internal_admin.identity_governance.operator_grant.revoked",
        &request.reason,
    )
    .await?;

    Ok(OperatorGrantActionResult {
        object_id: principal_id,
        principal_id,
        role,
        previous_status,
        next_status: "revoked".to_string(),
        audit_action: "internal_admin.identity_governance.operator_grant.revoked",
    })
}

async fn principal_tenant_id(db: &PgPool, principal_id: Uuid) -> Result<Uuid, AppError> {
    Ok(
        sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM principals WHERE id = $1")
            .bind(principal_id)
            .fetch_one(db)
            .await?,
    )
}

async fn current_grant_status(
    db: &PgPool,
    principal_id: Uuid,
    role: &str,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT status FROM internal_admin_operator_grants WHERE principal_id = $1 AND role = $2",
    )
    .bind(principal_id)
    .bind(role)
    .fetch_optional(db)
    .await?)
}

async fn insert_operator_grant_audit(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    principal_id: Uuid,
    role: &str,
    previous_status: Option<&str>,
    next_status: &str,
    action: &'static str,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'operator_grant', $4,
           jsonb_build_object(
             'reason', $5,
             'role', $6,
             'previous_status', $7,
             'next_status', $8
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(principal_id)
    .bind(reason.trim())
    .bind(role)
    .bind(previous_status)
    .bind(next_status)
    .execute(db)
    .await?;
    Ok(())
}
