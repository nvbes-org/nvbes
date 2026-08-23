use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn save_key(db: &PgPool, principal_id: Uuid, object_key: &str) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO account_profiles (principal_id, avatar_object_key)
        VALUES ($1, $2)
        ON CONFLICT (principal_id) DO UPDATE
        SET avatar_object_key = EXCLUDED.avatar_object_key,
            updated_at = NOW()
        "#,
    )
    .bind(principal_id)
    .bind(object_key)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn fetch_key(db: &PgPool, principal_id: Uuid) -> Result<Option<String>, AppError> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT avatar_object_key FROM account_profiles WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await
    .map(Option::flatten)
    .map_err(AppError::from)
}

pub async fn clear_key(db: &PgPool, principal_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE account_profiles SET avatar_object_key = NULL, updated_at = NOW() WHERE principal_id = $1",
    )
    .bind(principal_id)
    .execute(db)
    .await?;
    Ok(())
}
