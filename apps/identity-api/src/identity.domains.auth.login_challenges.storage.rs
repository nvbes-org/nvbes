use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    CachedLoginChallenge,
    keys::{active_challenge_key, auth_state_index_id, challenge_key, challenge_ttl_seconds},
};
use crate::http::error::AppError;

pub(super) async fn set_challenge(
    redis: &nvbes_redis::RedisPool,
    challenge: &CachedLoginChallenge,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_set_json(
            &challenge_key(&challenge.id.to_string()),
            challenge,
            challenge_ttl_seconds(challenge.expires_at),
        )
        .await
        .map_err(|err| AppError::internal("login_challenge_store_failed", format!("{}", err)))?;

    client
        .sadd(
            &auth_state_index_id(challenge.auth_state_id),
            &challenge.id.to_string(),
        )
        .await
        .map_err(|err| AppError::internal("login_challenge_store_failed", format!("{}", err)))?;
    client
        .expire(
            &auth_state_index_id(challenge.auth_state_id),
            challenge_ttl_seconds(challenge.expires_at) as i64,
        )
        .await
        .map_err(|err| AppError::internal("login_challenge_store_failed", format!("{}", err)))?;

    Ok(())
}

pub(super) async fn delete_challenge_by_id(
    redis: &nvbes_redis::RedisPool,
    challenge_id: &Uuid,
) -> Result<(), AppError> {
    if let Some(challenge) = super::state::get_challenge(redis, *challenge_id).await? {
        delete_challenge(redis, &challenge).await?;
    }
    Ok(())
}

pub(super) async fn delete_challenge(
    redis: &nvbes_redis::RedisPool,
    challenge: &CachedLoginChallenge,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .del_key(&challenge_key(&challenge.id.to_string()))
        .await
        .map_err(|err| AppError::internal("login_challenge_delete_failed", format!("{}", err)))?;
    remove_challenge_reference(redis, challenge.auth_state_id, &challenge.id.to_string()).await?;
    clear_active_challenge_if_current(
        redis,
        challenge.auth_state_id,
        &challenge.purpose,
        &challenge.id.to_string(),
    )
    .await
}

pub(super) async fn remove_challenge_reference(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    challenge_id: &str,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .srem(&auth_state_index_id(auth_state_id), challenge_id)
        .await
        .map_err(|err| {
            AppError::internal("login_challenge_index_delete_failed", format!("{}", err))
        })?;
    Ok(())
}

pub(super) async fn clear_active_challenge_if_current(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    purpose: &str,
    challenge_id: &str,
) -> Result<(), AppError> {
    let active_key = active_challenge_key(auth_state_id, purpose);
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let current = client
        .cache_get_json::<String>(&active_key)
        .await
        .map_err(|err| {
            AppError::internal("login_challenge_active_load_failed", format!("{}", err))
        })?;
    if current.as_deref() == Some(challenge_id) {
        client.del_key(&active_key).await.map_err(|err| {
            AppError::internal("login_challenge_active_delete_failed", format!("{}", err))
        })?;
    }
    Ok(())
}

pub(super) async fn set_active_challenge_id(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    purpose: &str,
    challenge_id: &Uuid,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let active_key = active_challenge_key(auth_state_id, purpose);
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_set_json(
            &active_key,
            &challenge_id.to_string(),
            challenge_ttl_seconds(expires_at),
        )
        .await
        .map_err(|err| {
            AppError::internal("login_challenge_active_store_failed", format!("{}", err))
        })
}

pub(super) async fn current_active_challenge_id(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    purpose: &str,
) -> Result<Option<Uuid>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let current = client
        .cache_get_json::<String>(&active_challenge_key(auth_state_id, purpose))
        .await
        .map_err(|err| {
            AppError::internal("login_challenge_active_load_failed", format!("{}", err))
        })?;

    Ok(current.and_then(|value| Uuid::parse_str(&value).ok()))
}
