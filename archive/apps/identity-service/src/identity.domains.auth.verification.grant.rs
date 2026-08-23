use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::http::error::AppError;

const PASSWORD_CHANGE_GRANT_PREFIX: &str = "nvbes:identity:step-up-grant:password-change";

pub(super) async fn store_password_change_grant(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1) as u64;
    nvbes_redis::RedisClient::new(redis.clone())
        .set_with_ttl(&grant_key(session_id), &user_id.to_string(), ttl)
        .await
        .map_err(|error| AppError::internal("step_up_grant_store_failed", error.to_string()))
}

pub(super) async fn require_password_change_grant(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    let granted_user = nvbes_redis::RedisClient::new(redis.clone())
        .get(&grant_key(session_id))
        .await
        .map_err(|error| AppError::internal("step_up_grant_read_failed", error.to_string()))?;
    if granted_user.as_deref() != Some(user_id.to_string().as_str()) {
        return Err(nvbes_core::auth::step_up_required_error().into());
    }
    Ok(())
}

pub(super) async fn clear_password_change_grant(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    require_password_change_grant(redis, user_id, session_id).await?;
    nvbes_redis::RedisClient::new(redis.clone())
        .del_key(&grant_key(session_id))
        .await
        .map_err(|error| AppError::internal("step_up_grant_delete_failed", error.to_string()))
}

fn grant_key(session_id: Uuid) -> String {
    format!("{PASSWORD_CHANGE_GRANT_PREFIX}:{session_id}")
}
