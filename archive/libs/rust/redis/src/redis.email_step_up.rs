use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

const EMAIL_STEP_UP_KEY_PREFIX: &str = "nvbes:identity:email-step-up";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEmailStepUpChallenge {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn store_email_step_up_challenge(
    pool: &RedisPool,
    challenge: &CachedEmailStepUpChallenge,
) -> Result<(), RedisError> {
    let ttl = (challenge.expires_at - Utc::now()).num_seconds().max(1) as u64;
    let mut connection = pool.get().await?;
    let payload = serde_json::to_string(challenge)?;
    let _: () = connection
        .set_ex(challenge_key(challenge.id), payload, ttl)
        .await?;
    Ok(())
}

pub async fn take_email_step_up_challenge(
    pool: &RedisPool,
    challenge_id: Uuid,
) -> Result<Option<CachedEmailStepUpChallenge>, RedisError> {
    let client = crate::RedisClient::new(pool.clone());
    client.cache_take_json(&challenge_key(challenge_id)).await
}

pub async fn consume_email_step_up_challenge(
    pool: &RedisPool,
    challenge_id: Uuid,
) -> Result<(), RedisError> {
    crate::RedisClient::new(pool.clone())
        .del_key(&challenge_key(challenge_id))
        .await
}

fn challenge_key(challenge_id: Uuid) -> String {
    format!("{EMAIL_STEP_UP_KEY_PREFIX}:{challenge_id}")
}
