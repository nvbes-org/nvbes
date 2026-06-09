use uuid::Uuid;

use super::{
    CachedLoginChallenge, LoginChallenge, MAX_FAILED_ATTEMPTS,
    keys::{challenge_key, factor_allowed},
    storage::delete_challenge,
};
use crate::http::error::AppError;

pub async fn fetch_active_challenge(
    redis: &nvbes_redis::RedisPool,
    challenge_id: Uuid,
    auth_state_id: Uuid,
    _principal_id: Uuid,
    purpose: &'static str,
) -> Result<LoginChallenge, AppError> {
    let challenge = get_challenge(redis, challenge_id).await?;
    let Some(challenge) = challenge else {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    };

    if challenge.id != challenge_id
        || challenge.auth_state_id != auth_state_id
        || challenge.purpose != purpose
        || challenge.consumed_at.is_some()
        || challenge.expires_at <= chrono::Utc::now()
        || challenge.failed_attempts >= MAX_FAILED_ATTEMPTS
        || !factor_allowed(&challenge.allowed_factor_types, "webauthn")
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    Ok(LoginChallenge {
        id: challenge.id,
        auth_state_id: challenge.auth_state_id,
        principal_id: challenge.principal_id,
        metadata: challenge.metadata,
        allowed_factor_types: challenge.allowed_factor_types,
        failed_attempts: challenge.failed_attempts,
    })
}

pub async fn record_failed_attempt(
    redis: &nvbes_redis::RedisPool,
    challenge_id: Uuid,
    auth_state_id: Uuid,
    principal_id: Uuid,
    purpose: &'static str,
) -> Result<i32, AppError> {
    let mut challenge = get_challenge(redis, challenge_id)
        .await?
        .ok_or_else(|| AppError::not_found("challenge_not_found", "Challenge not found."))?;

    if challenge.auth_state_id != auth_state_id
        || challenge.principal_id != Some(principal_id)
        || challenge.purpose != purpose
        || challenge.consumed_at.is_some()
        || challenge.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    challenge.failed_attempts += 1;
    super::storage::set_challenge(redis, &challenge).await?;
    Ok(challenge.failed_attempts)
}

pub async fn consume_challenge(
    redis: &nvbes_redis::RedisPool,
    challenge_id: Uuid,
    auth_state_id: Uuid,
    principal_id: Uuid,
    purpose: &'static str,
) -> Result<(), AppError> {
    let challenge = get_challenge(redis, challenge_id)
        .await?
        .ok_or_else(|| AppError::not_found("challenge_not_found", "Challenge not found."))?;

    if challenge.auth_state_id != auth_state_id
        || challenge.principal_id != Some(principal_id)
        || challenge.purpose != purpose
        || challenge.consumed_at.is_some()
        || challenge.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    delete_challenge(redis, &challenge).await?;
    Ok(())
}

pub async fn get_challenge(
    redis: &nvbes_redis::RedisPool,
    challenge_id: Uuid,
) -> Result<Option<CachedLoginChallenge>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_get_json(&challenge_key(&challenge_id.to_string()))
        .await
        .map_err(|err| AppError::internal("login_challenge_load_failed", format!("{}", err)))
}
