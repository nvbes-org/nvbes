use std::collections::BTreeSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::developer::{
        rbac::permissions_for_role,
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
    db: &PgPool,
    auth: &AuthContext,
) -> Result<DeveloperOverviewResponse, AppError> {
    let tenant_id = require_tenant_id(auth)?;
    let (
        oauth_clients,
        marketplace_pending,
        high_risk_scopes,
        failed_webhook_deliveries,
        unhealthy_integrations,
        active_sandboxes,
    ) = tokio::try_join!(
        count_oauth_clients(db, tenant_id),
        count_pending_marketplace_apps(db, tenant_id),
        count_high_risk_scopes(db),
        count_failed_webhook_deliveries(db, tenant_id),
        count_unhealthy_integrations(db, tenant_id),
        count_active_sandboxes(db, tenant_id),
    )?;

    Ok(DeveloperOverviewResponse {
        tenant_id,
        oauth_clients,
        marketplace_pending,
        high_risk_scopes,
        failed_webhook_deliveries,
        unhealthy_integrations,
        active_sandboxes,
    })
}

pub(crate) fn require_tenant_id(auth: &AuthContext) -> Result<Uuid, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_required",
            "Developer Console requires a tenant-scoped session",
        )
    })
}

async fn count_oauth_clients(db: &PgPool, tenant_id: Uuid) -> Result<i64, AppError> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM oauth_clients WHERE tenant_id = $1",
    )
    .await
}

async fn count_pending_marketplace_apps(db: &PgPool, tenant_id: Uuid) -> Result<i64, AppError> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_marketplace_apps WHERE tenant_id = $1 AND status = 'pending'",
    )
    .await
}

async fn count_high_risk_scopes(db: &PgPool) -> Result<i64, AppError> {
    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM developer_scope_registry WHERE risk IN ('high', 'restricted')",
    )
    .fetch_one(db)
    .await?;
    Ok(total)
}

async fn count_failed_webhook_deliveries(db: &PgPool, tenant_id: Uuid) -> Result<i64, AppError> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_webhook_deliveries WHERE tenant_id = $1 AND status = 'failed'",
    )
    .await
}

async fn count_unhealthy_integrations(db: &PgPool, tenant_id: Uuid) -> Result<i64, AppError> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_health_checks WHERE tenant_id = $1 AND status IN ('failing', 'warning')",
    )
    .await
}

async fn count_active_sandboxes(db: &PgPool, tenant_id: Uuid) -> Result<i64, AppError> {
    count(
        db,
        tenant_id,
        "SELECT COUNT(*) FROM developer_sandbox_tenants WHERE tenant_id = $1 AND status = 'active'",
    )
    .await
}

async fn count(db: &PgPool, tenant_id: Uuid, sql: &str) -> Result<i64, AppError> {
    let (total,): (i64,) = sqlx::query_as(sql).bind(tenant_id).fetch_one(db).await?;
    Ok(total)
}
