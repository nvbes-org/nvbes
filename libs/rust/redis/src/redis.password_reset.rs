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

pub async fn mark_password_reset_token_consumed(
    pool: &RedisPool,
    token_hash: &str,
) -> Result<(), RedisError> {
    if let Some(mut token) = get_password_reset_token(pool, token_hash).await? {
        token.consumed_at = Some(Utc::now());
        store_password_reset_token(pool, &token).await?;
    }
    Ok(())
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

    async fn test_redis_pool() -> RedisPool {
        let mut config = crate::config::RedisConfig::from_env();
        if config.url == "redis://localhost:6379" {
            config.url = "redis://127.0.0.1:6379".to_string();
        }
        config.max_connections = 2;
        crate::connection::create_pool(&config)
            .await
            .expect("redis pool")
    }

    #[tokio::test]
    async fn stores_and_consumes_password_reset_token() {
        let redis = test_redis_pool().await;
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

        mark_password_reset_token_consumed(&redis, &token_hash)
            .await
            .expect("token should be consumable");

        let token = get_password_reset_token(&redis, &token_hash)
            .await
            .expect("token lookup should work")
            .expect("token should exist");
        assert!(token.consumed_at.is_some());
    }
}
