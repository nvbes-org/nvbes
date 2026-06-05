use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::http::error::AppError;

#[path = "identity.domains.auth.login_challenges.tests.rs"]
mod tests;

const MAX_FAILED_ATTEMPTS: i32 = 5;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateLoginChallengeInput {
    pub auth_state_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub purpose: &'static str,
    pub required_level: &'static str,
    pub allowed_factor_types: Vec<&'static str>,
    pub factor_id: Option<Uuid>,
    pub metadata: Value,
    pub ttl_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLoginChallenge {
    pub id: Uuid,
    pub auth_state_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub purpose: String,
    pub required_level: String,
    pub allowed_factor_types: Vec<String>,
    pub factor_id: Option<Uuid>,
    pub metadata: Value,
    pub failed_attempts: i32,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct LoginChallenge {
    pub id: Uuid,
    pub auth_state_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub metadata: Value,
    pub allowed_factor_types: Vec<String>,
    pub failed_attempts: i32,
}

pub async fn prune_expired_challenges(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
) -> Result<u64, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let mut removed = 0;
    let challenge_ids: Vec<String> = client
        .smembers(&auth_state_index_id(auth_state_id))
        .await
        .map_err(|err| AppError::internal("login_challenge_prune_failed", &format!("{}", err)))?;

    for challenge_id in challenge_ids {
        let Some(challenge) = nvbes_redis::cache::cache_get_json::<CachedLoginChallenge>(
            redis,
            &challenge_key(&challenge_id),
        )
        .await
        .map_err(|err| AppError::internal("login_challenge_prune_failed", &format!("{}", err)))?
        else {
            remove_challenge_reference(redis, auth_state_id, &challenge_id).await?;
            continue;
        };

        if challenge.expires_at <= Utc::now() {
            delete_challenge(redis, &challenge).await?;
            removed += 1;
        }
    }

    Ok(removed)
}

pub async fn replace_challenge(
    redis: &nvbes_redis::RedisPool,
    input: CreateLoginChallengeInput,
) -> Result<Uuid, AppError> {
    let lock_key = format!("login-challenges:{}:{}", input.auth_state_id, input.purpose);
    let locked = nvbes_redis::lock::acquire(redis, &lock_key, 10)
        .await
        .map_err(|err| AppError::internal("login_challenge_replace_failed", &format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "challenge_locked",
            "The login challenge flow is locked.",
        ));
    }

    let result = async {
        let _ = prune_expired_challenges(redis, input.auth_state_id).await?;
        let current_active =
            current_active_challenge_id(redis, input.auth_state_id, input.purpose).await?;

        if let Some(current_active) = current_active {
            delete_challenge_by_id(redis, &current_active).await?;
        }

        let challenge_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::minutes(input.ttl_minutes.max(1));
        let allowed_factor_types = input
            .allowed_factor_types
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let challenge = CachedLoginChallenge {
            id: challenge_id,
            auth_state_id: input.auth_state_id,
            principal_id: input.principal_id,
            tenant_id: input.tenant_id,
            workspace_id: input.workspace_id,
            purpose: input.purpose.to_string(),
            required_level: input.required_level.to_string(),
            allowed_factor_types,
            factor_id: input.factor_id,
            metadata: input.metadata,
            failed_attempts: 0,
            expires_at,
            consumed_at: None,
        };

        set_challenge(redis, &challenge).await?;
        set_active_challenge_id(
            redis,
            input.auth_state_id,
            input.purpose,
            &challenge_id,
            expires_at,
        )
        .await?;
        Ok(challenge_id)
    }
    .await;

    let release_result = nvbes_redis::lock::release(redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("login_challenge_replace_failed", &format!("{}", err)));
    release_result?;

    result
}

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
        || challenge.expires_at <= Utc::now()
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
        || challenge.expires_at <= Utc::now()
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    challenge.failed_attempts += 1;
    set_challenge(redis, &challenge).await?;
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
        || challenge.expires_at <= Utc::now()
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
        .map_err(|err| AppError::internal("login_challenge_load_failed", &format!("{}", err)))
}

async fn set_challenge(
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
        .map_err(|err| AppError::internal("login_challenge_store_failed", &format!("{}", err)))?;

    client
        .sadd(
            &auth_state_index_id(challenge.auth_state_id),
            &challenge.id.to_string(),
        )
        .await
        .map_err(|err| AppError::internal("login_challenge_store_failed", &format!("{}", err)))?;
    client
        .expire(
            &auth_state_index_id(challenge.auth_state_id),
            challenge_ttl_seconds(challenge.expires_at) as i64,
        )
        .await
        .map_err(|err| AppError::internal("login_challenge_store_failed", &format!("{}", err)))?;

    Ok(())
}

async fn delete_challenge_by_id(
    redis: &nvbes_redis::RedisPool,
    challenge_id: &Uuid,
) -> Result<(), AppError> {
    if let Some(challenge) = get_challenge(redis, *challenge_id).await? {
        delete_challenge(redis, &challenge).await?;
    }
    Ok(())
}

async fn delete_challenge(
    redis: &nvbes_redis::RedisPool,
    challenge: &CachedLoginChallenge,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .del_key(&challenge_key(&challenge.id.to_string()))
        .await
        .map_err(|err| AppError::internal("login_challenge_delete_failed", &format!("{}", err)))?;
    remove_challenge_reference(redis, challenge.auth_state_id, &challenge.id.to_string()).await?;
    clear_active_challenge_if_current(
        redis,
        challenge.auth_state_id,
        &challenge.purpose,
        &challenge.id.to_string(),
    )
    .await
}

async fn remove_challenge_reference(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    challenge_id: &str,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .srem(&auth_state_index_id(auth_state_id), challenge_id)
        .await
        .map_err(|err| {
            AppError::internal("login_challenge_index_delete_failed", &format!("{}", err))
        })?;
    Ok(())
}

async fn clear_active_challenge_if_current(
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
            AppError::internal("login_challenge_active_load_failed", &format!("{}", err))
        })?;
    if current.as_deref() == Some(challenge_id) {
        client.del_key(&active_key).await.map_err(|err| {
            AppError::internal("login_challenge_active_delete_failed", &format!("{}", err))
        })?;
    }
    Ok(())
}

async fn set_active_challenge_id(
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
            AppError::internal("login_challenge_active_store_failed", &format!("{}", err))
        })
}

async fn current_active_challenge_id(
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    purpose: &str,
) -> Result<Option<Uuid>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let current = client
        .cache_get_json::<String>(&active_challenge_key(auth_state_id, purpose))
        .await
        .map_err(|err| {
            AppError::internal("login_challenge_active_load_failed", &format!("{}", err))
        })?;

    Ok(current.and_then(|value| Uuid::parse_str(&value).ok()))
}

fn challenge_key(challenge_id: &str) -> String {
    format!("nvbes:identity:login-challenge:{challenge_id}")
}

fn auth_state_index_id(auth_state_id: Uuid) -> String {
    format!("nvbes:identity:login-challenges:auth-state:{auth_state_id}")
}

fn active_challenge_key(auth_state_id: Uuid, purpose: &str) -> String {
    format!("nvbes:identity:login-challenges:active:{auth_state_id}:{purpose}")
}

fn factor_allowed(allowed_factor_types: &[String], factor_type: &str) -> bool {
    allowed_factor_types.is_empty()
        || allowed_factor_types
            .iter()
            .any(|value| value == factor_type)
}

fn challenge_ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}
