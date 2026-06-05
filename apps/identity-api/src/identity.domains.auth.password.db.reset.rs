use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn insert_password_reset_token(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    token_hash: String,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    nvbes_redis::password_reset::store_password_reset_token(
        redis,
        &nvbes_redis::password_reset::CachedPasswordResetToken {
            principal_id,
            token_hash,
            created_at: Utc::now(),
            expires_at,
            consumed_at: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("password_reset_token_store_failed", &err.to_string()))?;
    Ok(())
}

pub async fn insert_password_reset_token_tx(
    tx: &mut Transaction<'_, Postgres>,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    token_hash: String,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let _ = tx;
    insert_password_reset_token(redis, principal_id, token_hash, expires_at).await?;
    Ok(())
}

pub async fn upsert_password_reset_token(
    tx: &mut Transaction<'_, Postgres>,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let _ = tx;
    insert_password_reset_token(redis, principal_id, token_hash.to_string(), expires_at).await?;
    Ok(())
}

pub async fn find_token_for_reset(
    redis: &nvbes_redis::RedisPool,
    token_hash: &str,
) -> Result<Option<(Uuid, DateTime<Utc>, Option<DateTime<Utc>>)>, AppError> {
    nvbes_redis::password_reset::get_password_reset_token(redis, token_hash)
        .await
        .map_err(|err| AppError::internal("password_reset_token_read_failed", &err.to_string()))
        .map(|token| token.map(|token| (token.principal_id, token.expires_at, token.consumed_at)))
}

pub async fn apply_password_reset(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    token_hash: &str,
    password_hash: &str,
) -> Result<(), AppError> {
    let _ = token_hash;
    sqlx::query("UPDATE users SET password_hash = $2, password_last_changed_at = NOW(), updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .bind(password_hash)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
