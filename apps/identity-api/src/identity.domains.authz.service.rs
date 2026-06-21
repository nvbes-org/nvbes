use super::db::*;
use super::types::*;
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use axum::http::HeaderMap;
use nvbes_core::authz::{action_requires_step_up, is_allowed};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.authz.service.tenant.rs"]
mod tenant;

pub use tenant::{ensure_email_verified, ensure_tenant_management_access, resolve_admin_scope};

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
