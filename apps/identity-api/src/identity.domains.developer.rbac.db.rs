use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

use super::rbac::{DeveloperPermission, DeveloperRole, permissions_for_role};

pub async fn load_developer_roles(
    db: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Vec<DeveloperRole>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM developer_role_assignments
        WHERE tenant_id = $1
          AND principal_id = $2
          AND revoked_at IS NULL
        ORDER BY role::text
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|role| parse_developer_role(&role))
        .collect()
}

pub fn permissions_for_roles(roles: &[DeveloperRole]) -> Vec<DeveloperPermission> {
    let mut seen = HashSet::new();
    let mut permissions = Vec::new();

    for role in roles {
        for permission in permissions_for_role(*role) {
            if seen.insert(permission) {
                permissions.push(permission);
            }
        }
    }

    permissions
}

pub fn parse_developer_role(role: &str) -> Result<DeveloperRole, AppError> {
    match role {
        "developer_admin" => Ok(DeveloperRole::DeveloperAdmin),
        "app_manager" => Ok(DeveloperRole::AppManager),
        "webhook_manager" => Ok(DeveloperRole::WebhookManager),
        "log_viewer" => Ok(DeveloperRole::LogViewer),
        "integration_tester" => Ok(DeveloperRole::IntegrationTester),
        "docs_viewer" => Ok(DeveloperRole::DocsViewer),
        _ => Err(AppError::internal(
            "developer_role_unknown",
            format!("Unknown developer role: {role}."),
        )),
    }
}
