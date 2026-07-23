use super::{AdminScope, TenantManagementAuth};
use crate::http::error::AppError;
use nvbes_core::authz::{IdentityRole, parse_identity_role};
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

pub async fn resolve_admin_scope(
    db: &PgPool,
    auth: &impl TenantManagementAuth,
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
) -> Result<AdminScope, AppError> {
    ensure_tenant_context(auth, tenant_id)?;

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

pub fn ensure_tenant_context(
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

fn identity_role_allows_tenant_management(role: IdentityRole) -> bool {
    matches!(
        role,
        IdentityRole::Owner | IdentityRole::Admin | IdentityRole::SecurityAdmin
    )
}

#[cfg(test)]
#[path = "identity.domains.authz.service.tenant.tests.rs"]
mod tests;
