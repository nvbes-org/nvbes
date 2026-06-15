use super::db::*;
use super::types::*;
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use axum::http::HeaderMap;
use nvbes_core::authz::{IdentityRole, action_requires_step_up, is_allowed, parse_identity_role};
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

    let role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE tenant_id = $1
          AND principal_id = $2
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(auth.user_id())
    .fetch_optional(db)
    .await?;

    let allowed = role
        .as_deref()
        .and_then(parse_identity_role)
        .is_some_and(identity_role_allows_tenant_management);

    if !allowed {
        return Err(AppError::forbidden(
            "tenant_management_denied",
            "You do not have permission to manage this tenant.",
        ));
    }

    Ok(())
}

pub(super) fn identity_role_allows_tenant_management(role: IdentityRole) -> bool {
    matches!(
        role,
        IdentityRole::Owner | IdentityRole::Admin | IdentityRole::SecurityAdmin
    )
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

pub async fn resolve_admin_scope(
    db: &PgPool,
    auth: &impl TenantManagementAuth,
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
) -> Result<AdminScope, AppError> {
    ensure_tenant_context(auth, tenant_id)?;

    // 1. Check if tenant-level membership allows admin elevation
    let tenant_role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE tenant_id = $1
          AND principal_id = $2
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(auth.user_id())
    .fetch_optional(db)
    .await?;

    let is_tenant_admin = tenant_role
        .as_deref()
        .and_then(parse_identity_role)
        .is_some_and(identity_role_allows_tenant_management);

    if is_tenant_admin {
        return Ok(AdminScope::Tenant);
    }

    // 2. Check organization-level membership role
    if let Some(org_id) = organization_id {
        let belongs = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM organizations WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(org_id)
        .bind(tenant_id)
        .fetch_one(db)
        .await?;

        if belongs {
            let org_role = sqlx::query_scalar::<_, String>(
                r#"
                SELECT role::text
                FROM organization_memberships
                WHERE organization_id = $1
                  AND principal_id = $2
                  AND status = 'active'
                LIMIT 1
                "#,
            )
            .bind(org_id)
            .bind(auth.user_id())
            .fetch_optional(db)
            .await?;

            let is_org_admin = org_role
                .as_deref()
                .and_then(parse_identity_role)
                .is_some_and(identity_role_allows_tenant_management);

            if is_org_admin {
                return Ok(AdminScope::Organization(org_id));
            }
        }
    }

    Err(AppError::forbidden(
        "tenant_management_denied",
        "You do not have permission to manage this tenant or organization.",
    ))
}

#[cfg(test)]
mod tests {
    use crate::domains::authz::service::identity_role_allows_tenant_management;
    use nvbes_core::authz::IdentityRole;

    #[test]
    fn tenant_management_is_limited_to_identity_admin_roles() {
        assert!(identity_role_allows_tenant_management(IdentityRole::Owner));
        assert!(identity_role_allows_tenant_management(IdentityRole::Admin));
        assert!(identity_role_allows_tenant_management(
            IdentityRole::SecurityAdmin
        ));
        assert!(!identity_role_allows_tenant_management(
            IdentityRole::BillingAdmin
        ));
        assert!(!identity_role_allows_tenant_management(
            IdentityRole::Member
        ));
    }

    #[tokio::test]
    async fn test_resolve_admin_scope() {
        let pool = crate::test_support::shared_test_pool();
        crate::test_support::ensure_test_database(&pool).await;

        let tenant_id = uuid::Uuid::new_v4();
        let principal_id = uuid::Uuid::new_v4();
        let org_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        // 1. Insert tenant
        sqlx::query(
            r#"
            INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
            VALUES ($1, 'personal', 'Test Tenant', $2, 'active', 'standard', $3, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(format!("tenant-{}", tenant_id))
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // 2. Insert principal
        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at) VALUES ($1, $2, 'human', 'active', 'Test User', $3, $3)",
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // 3. Insert organization
        sqlx::query(
            "INSERT INTO organizations (id, tenant_id, name, slug, status, created_at, updated_at) VALUES ($1, $2, 'Test Org', $3, 'active', $4, $4)",
        )
        .bind(org_id)
        .bind(tenant_id)
        .bind(format!("org-{}", org_id))
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // Let's mock TenantManagementAuth
        struct MockAuth {
            user_id: uuid::Uuid,
            tenant_id: Option<uuid::Uuid>,
            email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
        }
        impl super::TenantManagementAuth for MockAuth {
            fn user_id(&self) -> uuid::Uuid { self.user_id }
            fn tenant_id(&self) -> Option<uuid::Uuid> { self.tenant_id }
            fn email_verified_at(&self) -> Option<chrono::DateTime<chrono::Utc>> { self.email_verified_at }
        }

        let auth = MockAuth {
            user_id: principal_id,
            tenant_id: Some(tenant_id),
            email_verified_at: Some(now),
        };

        // Initially no memberships: should return forbidden
        let result = super::resolve_admin_scope(&pool, &auth, tenant_id, Some(org_id)).await;
        assert!(result.is_err());

        // Add tenant membership as admin
        sqlx::query(
            "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at) VALUES ($1, $2, 'human', 'admin', 'active', $3, $3)",
        )
        .bind(tenant_id)
        .bind(principal_id)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // Now tenant admin
        let scope = super::resolve_admin_scope(&pool, &auth, tenant_id, Some(org_id)).await.unwrap();
        assert_eq!(scope, super::AdminScope::Tenant);

        // Delete tenant membership to test org admin
        sqlx::query("DELETE FROM tenant_memberships WHERE tenant_id = $1 AND principal_id = $2")
            .bind(tenant_id)
            .bind(principal_id)
            .execute(&pool)
            .await
            .unwrap();

        // Insert organization membership as admin
        sqlx::query(
            "INSERT INTO organization_memberships (organization_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'admin', 'active', $3, $3)",
        )
        .bind(org_id)
        .bind(principal_id)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // Now organization admin
        let scope = super::resolve_admin_scope(&pool, &auth, tenant_id, Some(org_id)).await.unwrap();
        assert_eq!(scope, super::AdminScope::Organization(org_id));
    }
}
