use super::db::*;
use super::types::*;
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use axum::http::HeaderMap;
use nvbes_core::authz::{action_requires_step_up, is_allowed};
use sqlx::PgPool;
use uuid::Uuid;

pub fn ensure_email_verified(auth: &impl TenantManagementAuth) -> Result<(), AppError> {
    if auth.email_verified_at().is_none() {
        return Err(AppError::forbidden(
            "email_not_verified",
            "Verify your email address before performing this action.",
        ));
    }
    Ok(())
}

pub async fn authorize_workspace_action(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
    workspace_id: Uuid,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceAccess, AppError> {
    let auth = authenticate_workspace_bearer(db, redis, jwt, headers).await?;
    let access = load_workspace_access(db, redis, &auth, workspace_id).await?;
    let decision = decide_loaded_workspace_action(redis, &auth, &access, action, resource).await?;

    if decision.allowed {
        return Ok(access);
    }

    let effective_resource =
        effective_resource(access.policy.member_can_create_share_links, resource);
    record_permission_denied(db, &access, action, effective_resource, headers).await?;

    Err(workspace_decision_error(&decision))
}

pub async fn decide_workspace_action(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
    workspace_id: Uuid,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceDecision, AppError> {
    let auth = authenticate_workspace_bearer(db, redis, jwt, headers).await?;
    let access = load_workspace_access(db, redis, &auth, workspace_id).await?;
    decide_loaded_workspace_action(redis, &auth, &access, action, resource).await
}

pub async fn ensure_tenant_management_access(
    db: &PgPool,
    auth: &impl TenantManagementAuth,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    ensure_tenant_context(auth, tenant_id)?;

    let privileged_membership = sqlx::query(
        r#"
        SELECT 1
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE w.tenant_id = $1
          AND wm.principal_id = $2
          AND wm.status = 'active'
          AND wm.role IN ('owner', 'admin')
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(auth.user_id())
    .fetch_optional(db)
    .await?;

    if privileged_membership.is_none() {
        return Err(AppError::forbidden(
            "tenant_management_denied",
            "You do not have permission to manage this tenant.",
        ));
    }

    Ok(())
}

async fn authenticate_workspace_bearer(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
) -> Result<AuthContext, AppError> {
    crate::domains::auth::sessions::authenticate_bearer(db, redis, jwt, headers).await
}

async fn decide_loaded_workspace_action(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    access: &WorkspaceAccess,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceDecision, AppError> {
    let requires_step_up = action_requires_step_up(action);
    let effective_resource =
        effective_resource(access.policy.member_can_create_share_links, resource);

    if auth.email_verified_at.is_none() {
        return Ok(workspace_decision(
            false,
            "email_not_verified",
            action,
            Some(access.role),
            requires_step_up,
        ));
    }

    if !is_allowed(access.role, action, effective_resource) {
        return Ok(workspace_decision(
            false,
            "permission_denied",
            action,
            Some(access.role),
            requires_step_up,
        ));
    }

    if requires_step_up
        && crate::domains::auth::verification::require_recent_step_up(redis, auth, None)
            .await
            .is_err()
    {
        return Ok(workspace_decision(
            false,
            "step_up_required",
            action,
            Some(access.role),
            true,
        ));
    }

    Ok(workspace_decision(
        true,
        "allowed",
        action,
        Some(access.role),
        requires_step_up,
    ))
}

fn effective_resource(
    member_share_links_enabled: bool,
    mut resource: ResourceContext,
) -> ResourceContext {
    if !resource.member_share_links_enabled {
        resource.member_share_links_enabled = member_share_links_enabled;
    }
    resource
}

fn workspace_decision(
    allowed: bool,
    reason: &str,
    action: WorkspaceAction,
    role: Option<WorkspaceRole>,
    requires_step_up: bool,
) -> WorkspaceDecision {
    WorkspaceDecision {
        allowed,
        reason: reason.to_string(),
        action: action.as_str().to_string(),
        role,
        requires_step_up,
    }
}

fn workspace_decision_error(decision: &WorkspaceDecision) -> AppError {
    match decision.reason.as_str() {
        "email_not_verified" => AppError::forbidden(
            "email_not_verified",
            "Verify your email address before using this workspace.",
        ),
        "step_up_required" => nvbes_core::auth::step_up_required_error().into(),
        _ => AppError::forbidden(
            "permission_denied",
            "You do not have permission to perform this action in this workspace.",
        ),
    }
}

fn ensure_tenant_context(
    auth: &impl TenantManagementAuth,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    if auth.email_verified_at().is_none() {
        return Err(AppError::forbidden(
            "email_not_verified",
            "Verify your email address before using this tenant.",
        ));
    }

    let current_tenant = auth.tenant_id().ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using this tenant.",
        )
    })?;

    if current_tenant != tenant_id {
        return Err(AppError::forbidden(
            "tenant_mismatch",
            "This tenant does not match the active tenant context.",
        ));
    }

    Ok(())
}
