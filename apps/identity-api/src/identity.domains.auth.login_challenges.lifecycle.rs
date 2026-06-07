use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{
    CachedLoginChallenge, CreateLoginChallengeInput,
    keys::auth_state_index_id,
    storage::{
        current_active_challenge_id, delete_challenge, delete_challenge_by_id,
        remove_challenge_reference, set_active_challenge_id, set_challenge,
    },
};
use crate::http::error::AppError;

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
            &super::keys::challenge_key(&challenge_id),
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
