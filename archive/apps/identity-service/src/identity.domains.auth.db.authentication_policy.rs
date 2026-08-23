use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn passwordless_login_enabled(db: &PgPool, principal_id: Uuid) -> Result<bool, AppError> {
    sqlx::query_scalar(
        "SELECT COALESCE((preferences ->> 'skip_password')::boolean, FALSE) FROM users WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await
    .map(|enabled| enabled.unwrap_or(false))
    .map_err(Into::into)
}

pub async fn enable_passwordless_login(db: &PgPool, principal_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE users SET preferences = jsonb_set(preferences, '{skip_password}', 'true'), updated_at = NOW() WHERE principal_id = $1",
    )
    .bind(principal_id)
    .execute(db)
    .await?;
    Ok(())
}
