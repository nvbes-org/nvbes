use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

const PASSWORD_RESET_TOKEN_KEY_PREFIX: &str = "nvbes:identity:password-reset-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPasswordResetToken {
    pub principal_id: Uuid,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

pub async fn store_password_reset_token(
    pool: &RedisPool,
    token: &CachedPasswordResetToken,
) -> Result<(), RedisError> {
    let ttl_seconds = ttl_seconds(token.expires_at);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(token)?;
    let _: () = conn
        .set_ex(
            password_reset_token_key(&token.token_hash),
            json,
            ttl_seconds,
        )
        .await?;
    Ok(())
}

pub async fn get_password_reset_token(
    pool: &RedisPool,
    token_hash: &str,
) -> Result<Option<CachedPasswordResetToken>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(password_reset_token_key(token_hash)).await?;
    raw.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

pub async fn take_password_reset_token(
    pool: &RedisPool,
    token_hash: &str,
) -> Result<Option<CachedPasswordResetToken>, RedisError> {
    let client = crate::RedisClient::new(pool.clone());
    client
        .cache_take_json(&password_reset_token_key(token_hash))
        .await
}

fn password_reset_token_key(token_hash: &str) -> String {
    format!("{PASSWORD_RESET_TOKEN_KEY_PREFIX}:{token_hash}")
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
    async fn stores_and_takes_password_reset_token() {
        let Some(redis) = test_redis_pool().await else {
            return;
        };
        let principal_id = Uuid::new_v4();
        let token_hash = format!("hash-{}", Uuid::new_v4());

        store_password_reset_token(
            &redis,
            &CachedPasswordResetToken {
                principal_id,
                token_hash: token_hash.clone(),
                created_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::minutes(5),
                consumed_at: None,
            },
        )
        .await
        .expect("token should store");

        let token = get_password_reset_token(&redis, &token_hash)
            .await
            .expect("token lookup should work")
            .expect("token should exist");
        assert_eq!(token.principal_id, principal_id);

        let taken = take_password_reset_token(&redis, &token_hash)
            .await
            .expect("token should be consumable")
            .expect("token should be returned");
        assert_eq!(taken.principal_id, principal_id);

        let token = get_password_reset_token(&redis, &token_hash)
            .await
            .expect("token lookup should work");
        assert!(token.is_none());
    }

    #[tokio::test]
    async fn takes_password_reset_token_once_under_concurrency() {
        let Some(redis) = test_redis_pool().await else {
            return;
        };
        let principal_id = Uuid::new_v4();
        let token_hash = format!("hash-{}", Uuid::new_v4());

        store_password_reset_token(
            &redis,
            &CachedPasswordResetToken {
                principal_id,
                token_hash: token_hash.clone(),
                created_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::minutes(5),
                consumed_at: None,
            },
        )
        .await
        .expect("token should store");

        let first = take_password_reset_token(&redis, &token_hash);
        let second = take_password_reset_token(&redis, &token_hash);
        let (first, second) = tokio::join!(first, second);

        let successful_takes = [first, second]
            .into_iter()
            .filter(|result| result.as_ref().is_ok_and(Option::is_some))
            .count();
        assert_eq!(successful_takes, 1);
    }
}
