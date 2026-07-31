use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn save_profile_avatar(
    db: &PgPool,
    principal_id: Uuid,
    object_key: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE users SET profile_avatar_key = $2, updated_at = NOW() WHERE principal_id = $1",
    )
    .bind(principal_id)
    .bind(object_key)
    .execute(db)
    .await
    .map_err(|_| AppError::internal("database_error", "Failed to save profile photo."))?;
    Ok(())
}

pub async fn fetch_profile_avatar(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Option<String>, AppError> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT profile_avatar_key FROM users WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::internal("database_error", "Failed to read profile photo."))?
    .map_or(Ok(None), Ok)
}

pub async fn clear_profile_avatar(db: &PgPool, principal_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE users SET profile_avatar_key = NULL, updated_at = NOW() WHERE principal_id = $1",
    )
    .bind(principal_id)
    .execute(db)
    .await
    .map_err(|_| AppError::internal("database_error", "Failed to remove profile photo."))?;
    Ok(())
}
