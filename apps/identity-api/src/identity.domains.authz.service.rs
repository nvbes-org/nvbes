use super::db::*;
use super::types::*;
use crate::domains::auth::jwt::JwtService;
use crate::http::error::AppError;
use crate::http::request::bearer_token;
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
    let token = bearer_token(headers)?;
    let auth = crate::domains::auth::sessions::authenticate(db, redis, jwt, &token).await?;
    let access = load_workspace_access(db, redis, &auth, workspace_id).await?;

    if auth.email_verified_at.is_none() {
        record_permission_denied(db, &access, action, resource, headers).await?;
        return Err(AppError::forbidden(
            "email_not_verified",
            "Verify your email address before using this workspace.",
        ));
    }

    let mut effective_resource = resource;
    if !effective_resource.member_share_links_enabled {
        effective_resource.member_share_links_enabled = access.policy.member_can_create_share_links;
    }

    if is_allowed(access.role, action, effective_resource) {
        if action_requires_step_up(action) {
            crate::domains::auth::verification::require_recent_step_up(redis, &auth, None).await?;
        }
        return Ok(access);
    }

    record_permission_denied(db, &access, action, effective_resource, headers).await?;

    Err(AppError::forbidden(
        "permission_denied",
        "You do not have permission to perform this action in this workspace.",
    ))
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
    let token = bearer_token(headers)?;
    let auth = crate::domains::auth::sessions::authenticate(db, redis, jwt, &token).await?;
    let access = load_workspace_access(db, redis, &auth, workspace_id).await?;
    let mut effective_resource = resource;
    if !effective_resource.member_share_links_enabled {
        effective_resource.member_share_links_enabled = access.policy.member_can_create_share_links;
    }

    if auth.email_verified_at.is_none() {
        return Ok(WorkspaceDecision {
            allowed: false,
            reason: "email_not_verified".to_string(),
            action: action.as_str().to_string(),
            role: Some(access.role),
            requires_step_up: action_requires_step_up(action),
        });
    }

    if !is_allowed(access.role, action, effective_resource) {
        return Ok(WorkspaceDecision {
            allowed: false,
            reason: "permission_denied".to_string(),
            action: action.as_str().to_string(),
            role: Some(access.role),
            requires_step_up: action_requires_step_up(action),
        });
    }

    if action_requires_step_up(action)
        && crate::domains::auth::verification::require_recent_step_up(redis, &auth, None)
            .await
            .is_err()
    {
        return Ok(WorkspaceDecision {
            allowed: false,
            reason: "step_up_required".to_string(),
            action: action.as_str().to_string(),
            role: Some(access.role),
            requires_step_up: true,
        });
    }

    Ok(WorkspaceDecision {
        allowed: true,
        reason: "allowed".to_string(),
        action: action.as_str().to_string(),
        role: Some(access.role),
        requires_step_up: action_requires_step_up(action),
    })
}

pub async fn ensure_tenant_management_access(
    db: &PgPool,
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
