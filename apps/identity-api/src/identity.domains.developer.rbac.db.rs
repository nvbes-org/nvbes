use sqlx::PgPool;
use uuid::Uuid;

use crate::{domains::developer::rbac::DeveloperRole, http::error::AppError};

pub async fn list_active_roles_for_principal(
    db: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Vec<DeveloperRole>, AppError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT role::text
        FROM developer_role_assignments
        WHERE tenant_id = $1
          AND principal_id = $2
          AND revoked_at IS NULL
        ORDER BY role::text ASC
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|(role,)| {
            DeveloperRole::try_from(role.as_str()).map_err(|err| {
                AppError::internal(
                    "developer_role_literal_invalid",
                    format!("Unknown developer role literal: {}", err.literal()),
                )
            })
        })
        .collect()
}
