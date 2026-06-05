use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::connection::{RedisError, RedisPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMessage {
    pub job_type: String,
    pub payload: String,
    pub idempotency_key: Option<String>,
    pub max_retries: u32,
    pub created_at: i64,
}

const QUEUE_KEY_PREFIX: &str = "nvbes:queue";

pub async fn enqueue(
    pool: &RedisPool,
    queue: &str,
    message: &JobMessage,
) -> Result<(), RedisError> {
    let key = format!("{QUEUE_KEY_PREFIX}:{queue}");
    let json = serde_json::to_string(message)?;
    let mut conn = pool.get().await?;
    let _: () = conn.rpush(&key, &json).await?;
    Ok(())
}

pub async fn enqueue_email(
    pool: &RedisPool,
    payload: &str,
    idempotency_key: Option<&str>,
) -> Result<(), RedisError> {
    let message = JobMessage {
        job_type: "email.send".to_string(),
        payload: payload.to_string(),
        idempotency_key: idempotency_key.map(|s| s.to_string()),
        max_retries: 3,
        created_at: chrono::Utc::now().timestamp(),
    };
    enqueue(pool, "email", &message).await
}

pub async fn enqueue_webhook(
    pool: &RedisPool,
    provider: &str,
    payload: &str,
    idempotency_key: Option<&str>,
) -> Result<(), RedisError> {
    let message = JobMessage {
        job_type: format!("webhook.{provider}"),
        payload: payload.to_string(),
        idempotency_key: idempotency_key.map(|s| s.to_string()),
        max_retries: 3,
        created_at: chrono::Utc::now().timestamp(),
    };
    enqueue(pool, "webhook", &message).await
}

pub async fn enqueue_maintenance(
    pool: &RedisPool,
    job_type: &str,
    payload: &str,
) -> Result<(), RedisError> {
    let message = JobMessage {
        job_type: format!("maintenance.{job_type}"),
        payload: payload.to_string(),
        idempotency_key: None,
        max_retries: 1,
        created_at: chrono::Utc::now().timestamp(),
    };
    enqueue(pool, "maintenance", &message).await
}

pub async fn enqueue_privacy(
    pool: &RedisPool,
    job_type: &str,
    workspace_id: &str,
) -> Result<(), RedisError> {
    let message = JobMessage {
        job_type: format!("privacy.{job_type}"),
        payload: workspace_id.to_string(),
        idempotency_key: None,
        max_retries: 3,
        created_at: chrono::Utc::now().timestamp(),
    };
    enqueue(pool, "privacy", &message).await
}

pub async fn dequeue(
    pool: &RedisPool,
    queue: &str,
    timeout_seconds: u64,
) -> Result<Option<JobMessage>, RedisError> {
    let key = format!("{QUEUE_KEY_PREFIX}:{queue}");
    let mut conn = pool.get().await?;

    let result: Option<(String, String)> = conn.blpop(&key, timeout_seconds as f64).await?;

    match result {
        Some((_, json)) => {
            let message = serde_json::from_str(&json)?;
            Ok(Some(message))
        }
        None => Ok(None),
    }
}

pub async fn get_queue_length(pool: &RedisPool, queue: &str) -> Result<u64, RedisError> {
    let key = format!("{QUEUE_KEY_PREFIX}:{queue}");
    let mut conn = pool.get().await?;
    let len: u64 = conn.llen(&key).await?;
    Ok(len)
}

pub async fn clear_queue(pool: &RedisPool, queue: &str) -> Result<(), RedisError> {
    let key = format!("{QUEUE_KEY_PREFIX}:{queue}");
    let mut conn = pool.get().await?;
    let _: () = conn.del(&key).await?;
    Ok(())
}
