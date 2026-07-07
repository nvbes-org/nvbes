use std::collections::BTreeSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::developer::{
        grpc,
        rbac::{DeveloperPermission, permissions_for_role},
        rbac_db,
        types::{DeveloperContextResponse, DeveloperOverviewResponse},
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn get_developer_context(
    db: &PgPool,
    auth: &AuthContext,
) -> Result<DeveloperContextResponse, AppError> {
    let tenant_id = require_tenant_id(auth)?;
    let roles = rbac_db::list_active_roles_for_principal(db, tenant_id, auth.user_id).await?;
    let mut permissions = BTreeSet::new();

    for role in &roles {
        for permission in permissions_for_role(*role) {
            permissions.insert(permission.as_api_str().to_string());
        }
    }

    Ok(DeveloperContextResponse {
        tenant_id,
        principal_id: auth.user_id,
        display_name: auth.display_name.clone(),
        email: auth.user_email.clone(),
        roles: roles
            .into_iter()
            .map(|role| role.as_db_str().to_string())
            .collect(),
        permissions: permissions.into_iter().collect(),
    })
}

pub async fn get_developer_overview(
    _db: &PgPool,
    auth: &AuthContext,
) -> Result<DeveloperOverviewResponse, AppError> {
    let tenant_id = require_tenant_id(auth)?;
    grpc::get_overview_summary(tenant_id, auth.user_id).await
}

pub(crate) fn require_tenant_id(auth: &AuthContext) -> Result<Uuid, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_required",
            "Developer Console requires a tenant-scoped session",
        )
    })
}

pub(crate) async fn require_permission(
    db: &PgPool,
    auth: &AuthContext,
    permission: DeveloperPermission,
) -> Result<Uuid, AppError> {
    let tenant_id = require_tenant_id(auth)?;
    let roles = rbac_db::list_active_roles_for_principal(db, tenant_id, auth.user_id).await?;
    let allowed = roles
        .into_iter()
        .flat_map(permissions_for_role)
        .any(|candidate| candidate == permission);

    if !allowed {
        return Err(AppError::forbidden(
            "developer_permission_required",
            format!("Developer permission required: {}", permission.as_api_str()),
        ));
    }

    Ok(tenant_id)
}
