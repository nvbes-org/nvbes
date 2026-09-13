use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

const AUTH_CHALLENGE_KEY_PREFIX: &str = "nvbes:identity:auth-challenge";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAuthChallenge {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub purpose: String,
    pub required_level: String,
    pub allowed_factor_types: Vec<String>,
    pub factor_id: Option<Uuid>,
    pub metadata: Value,
    pub expires_at: DateTime<Utc>,
}

pub async fn store_auth_challenge(
    pool: &RedisPool,
    challenge: &CachedAuthChallenge,
) -> Result<(), RedisError> {
    let ttl_seconds = ttl_seconds(challenge.expires_at);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(challenge)?;
    let _: () = conn
        .set_ex(auth_challenge_key(&challenge.id), json, ttl_seconds)
        .await?;
    Ok(())
}

pub async fn get_auth_challenge(
    pool: &RedisPool,
    challenge_id: Uuid,
) -> Result<Option<CachedAuthChallenge>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(auth_challenge_key(&challenge_id)).await?;
    raw.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

pub async fn take_auth_challenge(
    pool: &RedisPool,
    challenge_id: Uuid,
) -> Result<Option<CachedAuthChallenge>, RedisError> {
    let client = crate::RedisClient::new(pool.clone());
    client
        .cache_take_json(&auth_challenge_key(&challenge_id))
        .await
}

pub async fn consume_auth_challenge(
    pool: &RedisPool,
    challenge_id: Uuid,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: () = conn.del(auth_challenge_key(&challenge_id)).await?;
    Ok(())
}

fn auth_challenge_key(challenge_id: &Uuid) -> String {
    format!("{AUTH_CHALLENGE_KEY_PREFIX}:{challenge_id}")
}

fn ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    async fn test_redis_pool() -> Option<RedisPool> {
        nvbes_test_utils::redis::test_redis_pool().await
    }

    #[tokio::test]
    async fn takes_auth_challenge_once() {
        let Some(redis) = test_redis_pool().await else {
            return;
        };
        let challenge_id = Uuid::new_v4();
        let challenge = CachedAuthChallenge {
            id: challenge_id,
            principal_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            workspace_id: None,
            purpose: "webauthn_step_up".to_string(),
            required_level: "aal2".to_string(),
            allowed_factor_types: vec!["webauthn".to_string()],
            factor_id: None,
            metadata: serde_json::json!({
                "authentication": {
                    "state": "ok",
                },
            }),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        };

        store_auth_challenge(&redis, &challenge)
            .await
            .expect("challenge should store");

        let stored = take_auth_challenge(&redis, challenge_id)
            .await
            .expect("challenge take should work")
            .expect("challenge should exist");
        assert_eq!(stored.id, challenge_id);
        assert_eq!(stored.purpose, "webauthn_step_up");

        let stored = get_auth_challenge(&redis, challenge_id)
            .await
            .expect("challenge lookup should work");
        assert!(stored.is_none());
    }
}
