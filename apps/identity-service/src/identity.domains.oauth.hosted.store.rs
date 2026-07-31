use super::{
    hosted_keys::{hosted_authorization_state_key, hosted_authorization_state_ttl_seconds},
    hosted_types::CachedHostedAuthorizationState,
};
use crate::http::error::AppError;

pub async fn set_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    state: &CachedHostedAuthorizationState,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_set_json(
            &hosted_authorization_state_key(&state.state_id),
            state,
            hosted_authorization_state_ttl_seconds(),
        )
        .await
        .map_err(|err| {
            AppError::internal(
                "hosted_authorization_state_store_failed",
                format!("{}", err),
            )
        })
}

pub async fn get_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<Option<CachedHostedAuthorizationState>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_get_json(&hosted_authorization_state_key(state_id))
        .await
        .map_err(|err| {
            AppError::internal("hosted_authorization_state_load_failed", format!("{}", err))
        })
}

pub async fn delete_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .del_key(&hosted_authorization_state_key(state_id))
        .await
        .map_err(|err| {
            AppError::internal(
                "hosted_authorization_state_delete_failed",
                format!("{}", err),
            )
        })
}
