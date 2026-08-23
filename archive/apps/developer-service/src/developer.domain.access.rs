use sqlx::Row;

use crate::{
    http::{auth::DeveloperAuth, error::AppError},
    rbac::{DeveloperPermission, DeveloperRole, permissions_for_role},
};

pub async fn roles(
    db: &sqlx::PgPool,
    auth: &DeveloperAuth,
) -> Result<Vec<DeveloperRole>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT role::text AS role
        FROM developer_role_assignments
        WHERE tenant_id = $1
          AND principal_id = $2
          AND revoked_at IS NULL
        ORDER BY assigned_at ASC
        "#,
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            let literal: String = row.get("role");
            DeveloperRole::try_from(literal.as_str()).map_err(|_| {
                AppError::internal(
                    "developer_role_unknown",
                    format!("Unknown developer role: {literal}."),
                )
            })
        })
        .collect()
}

pub async fn require_permission(
    db: &sqlx::PgPool,
    auth: &DeveloperAuth,
    required: DeveloperPermission,
) -> Result<(), AppError> {
    let allowed = roles(db, auth)
        .await?
        .into_iter()
        .flat_map(permissions_for_role)
        .any(|permission| permission == required);

    if !allowed {
        return Err(AppError::forbidden(
            "developer_permission_required",
            format!("Developer permission required: {}", required.as_api_str()),
        ));
    }
    Ok(())
}
