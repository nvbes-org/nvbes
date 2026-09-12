use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool, command_with_timeout};

const EMAIL_VERIFICATION_TOKEN_KEY_PREFIX: &str = "nvbes:identity:email-verification-token";
const EMAIL_VERIFICATION_PRINCIPAL_INDEX_PREFIX: &str =
    "nvbes:identity:email-verification-tokens:principal";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEmailVerificationToken {
    pub principal_id: Uuid,
    #[serde(default)]
    pub email_address_id: Option<Uuid>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default = "default_purpose")]
    pub purpose: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

fn default_purpose() -> String {
    "primary_email".to_string()
}

pub async fn store_email_verification_token(
    pool: &RedisPool,
    token: &CachedEmailVerificationToken,
) -> Result<(), RedisError> {
    let ttl_seconds = ttl_seconds(token.expires_at);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(token)?;

    let _: () = command_with_timeout(
        "email_verification_token_set",
        conn.set_ex(
            email_verification_token_key(&token.token_hash),
            json,
            ttl_seconds,
        ),
    )
    .await?;
    let _: bool = command_with_timeout(
        "email_verification_principal_index_add",
        conn.sadd(
            email_verification_principal_index_key(&token.principal_id),
            &token.token_hash,
        ),
    )
    .await?;
    let _: bool = command_with_timeout(
        "email_verification_principal_index_expire",
        conn.expire(
            email_verification_principal_index_key(&token.principal_id),
            ttl_seconds as i64,
        ),
    )
    .await?;
    Ok(())
}

pub async fn get_email_verification_token(
    pool: &RedisPool,
    token_hash: &str,
) -> Result<Option<CachedEmailVerificationToken>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = command_with_timeout(
        "email_verification_token_get",
        conn.get(email_verification_token_key(token_hash)),
    )
    .await?;
    raw.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

pub async fn latest_unconsumed_email_verification_token_for_principal(
    pool: &RedisPool,
    principal_id: Uuid,
) -> Result<Option<CachedEmailVerificationToken>, RedisError> {
    let mut conn = pool.get().await?;
    let token_hashes: Vec<String> = command_with_timeout(
        "email_verification_principal_index_members",
        conn.smembers(email_verification_principal_index_key(&principal_id)),
    )
    .await?;

    let mut latest: Option<CachedEmailVerificationToken> = None;
    for token_hash in token_hashes {
        if let Some(token) = get_email_verification_token(pool, &token_hash).await?
            && token.consumed_at.is_none()
        {
            match latest {
                Some(ref current) if current.created_at >= token.created_at => {}
                _ => latest = Some(token),
            }
        }
    }

    Ok(latest)
}

pub async fn mark_email_verification_token_consumed(
    pool: &RedisPool,
    token_hash: &str,
) -> Result<(), RedisError> {
    if let Some(mut token) = get_email_verification_token(pool, token_hash).await? {
        token.consumed_at = Some(Utc::now());
        store_email_verification_token(pool, &token).await?;
    }
    Ok(())
}

pub async fn consume_all_email_verification_tokens_for_principal(
    pool: &RedisPool,
    principal_id: Uuid,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let token_hashes: Vec<String> = command_with_timeout(
        "email_verification_principal_index_members",
        conn.smembers(email_verification_principal_index_key(&principal_id)),
    )
    .await?;
    for token_hash in token_hashes {
        mark_email_verification_token_consumed(pool, &token_hash).await?;
    }
    Ok(())
}

fn email_verification_token_key(token_hash: &str) -> String {
    format!("{EMAIL_VERIFICATION_TOKEN_KEY_PREFIX}:{token_hash}")
}

fn email_verification_principal_index_key(principal_id: &Uuid) -> String {
    format!("{EMAIL_VERIFICATION_PRINCIPAL_INDEX_PREFIX}:{principal_id}")
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
    async fn stores_and_consumes_email_verification_token() {
        let Some(redis) = test_redis_pool().await else {
            return;
        };
        let principal_id = Uuid::new_v4();
        let token_hash = format!("hash-{}", Uuid::new_v4());

        store_email_verification_token(
            &redis,
            &CachedEmailVerificationToken {
                principal_id,
                email_address_id: None,
                email: None,
                purpose: default_purpose(),
                token_hash: token_hash.clone(),
                created_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::minutes(5),
                consumed_at: None,
            },
        )
        .await
        .expect("token should store");

        let token = latest_unconsumed_email_verification_token_for_principal(&redis, principal_id)
            .await
            .expect("token lookup should work")
            .expect("token should exist");
        assert_eq!(token.token_hash, token_hash);

        mark_email_verification_token_consumed(&redis, &token_hash)
            .await
            .expect("token should be consumable");

        let token = get_email_verification_token(&redis, &token_hash)
            .await
            .expect("token lookup should work")
            .expect("token should exist");
        assert!(token.consumed_at.is_some());
    }
}
