use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn revoke_developer_secret_version(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    version_id: Uuid,
) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>(
        r#"
        UPDATE developer_client_secret_versions
        SET status = 'revoked',
            revoked_at = NOW()
        WHERE tenant_id = $1
          AND id = $2
          AND revoked_at IS NULL
        RETURNING client_id
        "#,
    )
    .bind(tenant_id)
    .bind(version_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::not_found("developer_secret_not_found", "Developer secret not found."))
}

#[cfg(test)]
#[path = "identity.domains.enterprise.db.writes.tests.rs"]
mod tests;
