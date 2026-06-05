use std::time::Duration;

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

const KEY_PREFIX: &str = "nvbes:worker_queue";
const STATUS_PENDING: &str = "pending";
const STATUS_RUNNING: &str = "running";
const STATUS_FAILED: &str = "failed";
const STATUS_DEAD_LETTER: &str = "dead_letter";
const STATUS_SUCCEEDED: &str = "succeeded";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedJob {
    pub id: Uuid,
    pub queue: String,
    pub job_type: String,
    pub idempotency_key: Option<String>,
    pub payload: Value,
    pub max_attempts: u32,
    pub attempts: u32,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub claimed_at: Option<i64>,
    pub available_at: i64,
    pub last_error: Option<String>,
    pub result: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct QueueStatusEntry {
    pub status: String,
    pub depth: i64,
    pub oldest_age_seconds: Option<f64>,
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn queue_key(queue: &str, suffix: &str) -> String {
    format!("{KEY_PREFIX}:{queue}:{suffix}")
}

fn job_key(queue: &str, job_id: Uuid) -> String {
    queue_key(queue, &format!("job:{job_id}"))
}

fn dedupe_key(queue: &str, job_type: &str, idempotency_key: &str) -> String {
    queue_key(queue, &format!("dedupe:{job_type}:{idempotency_key}"))
}

fn pending_key(queue: &str) -> String {
    queue_key(queue, "pending")
}

fn running_key(queue: &str) -> String {
    queue_key(queue, "running")
}

fn delayed_key(queue: &str) -> String {
    queue_key(queue, "delayed")
}

fn dead_letter_key(queue: &str) -> String {
    queue_key(queue, "dead_letter")
}

async fn load_job(pool: &RedisPool, queue: &str, job_id: Uuid) -> Result<QueuedJob, RedisError> {
    let mut conn = pool.get().await?;
    let raw: String = conn.get(job_key(queue, job_id)).await?;
    Ok(serde_json::from_str(&raw)?)
}

pub async fn get_job(
    pool: &RedisPool,
    queue: &str,
    job_id: Uuid,
) -> Result<Option<QueuedJob>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(job_key(queue, job_id)).await?;
    raw.map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(Into::into)
}

pub async fn find_latest_job<F>(
    pool: &RedisPool,
    queue: &str,
    mut matches: F,
) -> Result<Option<QueuedJob>, RedisError>
where
    F: FnMut(&QueuedJob) -> bool,
{
    let mut conn = pool.get().await?;
    let job_keys: Vec<String> = redis::cmd("KEYS")
        .arg(queue_key(queue, "job:*"))
        .query_async(&mut *conn)
        .await?;

    let mut latest: Option<QueuedJob> = None;
    for key in job_keys {
        let raw: String = match conn.get(&key).await {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let Ok(job) = serde_json::from_str::<QueuedJob>(&raw) else {
            continue;
        };
        if !matches(&job) {
            continue;
        }

        match latest {
            Some(ref current) if current.created_at >= job.created_at => {}
            _ => latest = Some(job),
        }
    }

    Ok(latest)
}

async fn save_job(pool: &RedisPool, job: &QueuedJob) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let raw = serde_json::to_string(job)?;
    let _: () = conn.set(job_key(&job.queue, job.id), raw).await?;
    Ok(())
}

async fn move_to_pending(pool: &RedisPool, job: &QueuedJob) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: usize = conn
        .rpush(pending_key(&job.queue), job.id.to_string())
        .await?;
    Ok(())
}

async fn remove_from_indexes(pool: &RedisPool, job: &QueuedJob) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: usize = conn
        .zrem(running_key(&job.queue), job.id.to_string())
        .await?;
    let _: usize = conn
        .zrem(delayed_key(&job.queue), job.id.to_string())
        .await?;
    Ok(())
}

fn job_age_seconds(job: &QueuedJob, status: &str) -> Option<f64> {
    let origin = match status {
        STATUS_RUNNING => job.claimed_at,
        STATUS_FAILED => Some(job.available_at),
        STATUS_PENDING => Some(job.created_at),
        STATUS_DEAD_LETTER | STATUS_SUCCEEDED => Some(job.updated_at),
        _ => None,
    }?;
    Some((now_ts() - origin).max(0) as f64)
}

pub async fn enqueue_job(
    pool: &RedisPool,
    queue: &str,
    job_type: &str,
    payload: Value,
    idempotency_key: Option<&str>,
    max_attempts: u32,
    overwrite_terminal: bool,
    job_id: Option<Uuid>,
) -> Result<Uuid, RedisError> {
    let now = now_ts();
    let job_id = job_id.unwrap_or_else(Uuid::new_v4);
    let key = idempotency_key.map(|key| dedupe_key(queue, job_type, key));

    if let Some(dedupe_key) = key.as_deref() {
        let mut conn = pool.get().await?;
        let existing_id: Option<String> = conn.get(dedupe_key).await?;
        if let Some(existing_id) = existing_id {
            let existing_job_id = Uuid::parse_str(&existing_id)
                .map_err(|error| RedisError::Connection(error.to_string()))?;
            let mut job = load_job(pool, queue, existing_job_id).await?;
            if overwrite_terminal
                && matches!(job.status.as_str(), STATUS_FAILED | STATUS_DEAD_LETTER)
            {
                job.payload = payload;
                job.max_attempts = max_attempts;
                job.attempts = 0;
                job.status = STATUS_PENDING.to_string();
                job.created_at = now;
                job.updated_at = now;
                job.claimed_at = None;
                job.available_at = now;
                job.last_error = None;
                job.result = None;
                save_job(pool, &job).await?;
                move_to_pending(pool, &job).await?;
            }
            return Ok(existing_job_id);
        }
    }

    let job = QueuedJob {
        id: job_id,
        queue: queue.to_string(),
        job_type: job_type.to_string(),
        idempotency_key: idempotency_key.map(ToOwned::to_owned),
        payload,
        max_attempts,
        attempts: 0,
        status: STATUS_PENDING.to_string(),
        created_at: now,
        updated_at: now,
        claimed_at: None,
        available_at: now,
        last_error: None,
        result: None,
    };

    save_job(pool, &job).await?;
    move_to_pending(pool, &job).await?;

    if let Some(dedupe_key) = key.as_deref() {
        let mut conn = pool.get().await?;
        let _: () = conn.set(dedupe_key, job_id.to_string()).await?;
    }

    Ok(job_id)
}

pub async fn promote_due_jobs(pool: &RedisPool, queue: &str) -> Result<u64, RedisError> {
    let now = now_ts();
    let mut conn = pool.get().await?;
    let ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(delayed_key(queue))
        .arg("-inf")
        .arg(now)
        .query_async(&mut *conn)
        .await?;

    let mut promoted = 0_u64;
    for id in ids {
        let _: usize = conn.zrem(delayed_key(queue), &id).await?;
        let _: usize = conn.rpush(pending_key(queue), &id).await?;
        if let Ok(job_id) = Uuid::parse_str(&id)
            && let Ok(mut job) = load_job(pool, queue, job_id).await
        {
            job.status = STATUS_PENDING.to_string();
            job.available_at = now;
            job.updated_at = now;
            save_job(pool, &job).await?;
            promoted += 1;
        }
    }

    Ok(promoted)
}

pub async fn claim_next_job(
    pool: &RedisPool,
    queues: &[&str],
    timeout_seconds: u64,
) -> Result<Option<QueuedJob>, RedisError> {
    for queue in queues {
        let _ = promote_due_jobs(pool, queue).await?;
    }

    let mut conn = pool.get().await?;
    let mut cmd = redis::cmd("BLPOP");
    for queue in queues {
        cmd.arg(pending_key(queue));
    }
    cmd.arg(timeout_seconds);

    let result: Option<(String, String)> = cmd.query_async(&mut *conn).await?;
    let Some((key, job_id_raw)) = result else {
        return Ok(None);
    };

    let queue = key
        .strip_prefix(&format!("{KEY_PREFIX}:"))
        .and_then(|value| value.strip_suffix(":pending"))
        .ok_or_else(|| RedisError::Connection("unexpected queue key".to_string()))?;
    let job_id =
        Uuid::parse_str(&job_id_raw).map_err(|error| RedisError::Connection(error.to_string()))?;
    let mut job = load_job(pool, queue, job_id).await?;
    let now = now_ts();
    job.status = STATUS_RUNNING.to_string();
    job.attempts = job.attempts.saturating_add(1);
    job.claimed_at = Some(now);
    job.updated_at = now;
    job.last_error = None;
    save_job(pool, &job).await?;

    let mut conn = pool.get().await?;
    let _: usize = conn
        .zadd(running_key(queue), job.id.to_string(), now)
        .await?;
    Ok(Some(job))
}

pub async fn mark_job_succeeded(
    pool: &RedisPool,
    job: &QueuedJob,
    result: Value,
) -> Result<(), RedisError> {
    let mut finished = job.clone();
    let now = now_ts();
    finished.status = STATUS_SUCCEEDED.to_string();
    finished.updated_at = now;
    finished.result = Some(result);
    save_job(pool, &finished).await?;
    remove_from_indexes(pool, &finished).await?;
    Ok(())
}

pub async fn mark_job_failed(
    pool: &RedisPool,
    job: &QueuedJob,
    error: &str,
    retryable: bool,
    retry_delay: Duration,
) -> Result<(), RedisError> {
    let now = now_ts();
    let mut failed = job.clone();
    failed.updated_at = now;
    failed.last_error = Some(error.to_string());
    failed.claimed_at = None;

    let has_retry_budget = retryable && failed.attempts < failed.max_attempts;
    if has_retry_budget {
        failed.status = STATUS_FAILED.to_string();
        failed.available_at = now + retry_delay.as_secs() as i64;
        save_job(pool, &failed).await?;
        remove_from_indexes(pool, &failed).await?;
        let mut conn = pool.get().await?;
        let _: usize = conn
            .zadd(
                delayed_key(&failed.queue),
                failed.id.to_string(),
                failed.available_at,
            )
            .await?;
        return Ok(());
    }

    failed.status = STATUS_DEAD_LETTER.to_string();
    failed.result = None;
    save_job(pool, &failed).await?;
    remove_from_indexes(pool, &failed).await?;
    let mut conn = pool.get().await?;
    let _: usize = conn
        .sadd(dead_letter_key(&failed.queue), failed.id.to_string())
        .await?;
    Ok(())
}

pub async fn recover_stale_jobs(
    pool: &RedisPool,
    queue: &str,
    stale_after: Duration,
    retry_delay: Duration,
) -> Result<Vec<(String, String)>, RedisError> {
    let cutoff = now_ts() - stale_after.as_secs() as i64;
    let mut conn = pool.get().await?;
    let stale_ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(running_key(queue))
        .arg("-inf")
        .arg(cutoff)
        .query_async(&mut *conn)
        .await?;

    let mut recovered = Vec::new();
    for id in stale_ids {
        let Ok(job_id) = Uuid::parse_str(&id) else {
            continue;
        };
        let Ok(mut job) = load_job(pool, queue, job_id).await else {
            continue;
        };
        if job.status != STATUS_RUNNING {
            continue;
        }

        let outcome = if job.attempts >= job.max_attempts {
            STATUS_DEAD_LETTER
        } else {
            STATUS_FAILED
        };
        job.status = outcome.to_string();
        job.available_at = now_ts() + retry_delay.as_secs() as i64;
        job.updated_at = now_ts();
        job.last_error = Some("job timed out".to_string());
        job.claimed_at = None;
        save_job(pool, &job).await?;
        remove_from_indexes(pool, &job).await?;

        if outcome == STATUS_FAILED {
            let mut conn = pool.get().await?;
            let _: usize = conn
                .zadd(delayed_key(queue), job.id.to_string(), job.available_at)
                .await?;
        } else {
            let mut conn = pool.get().await?;
            let _: usize = conn
                .sadd(dead_letter_key(queue), job.id.to_string())
                .await?;
        }

        recovered.push((job.job_type, outcome.to_string()));
    }

    Ok(recovered)
}

pub async fn queue_status(
    pool: &RedisPool,
    queue: &str,
) -> Result<Vec<QueueStatusEntry>, RedisError> {
    let mut conn = pool.get().await?;
    let pending_depth: i64 = conn.llen(pending_key(queue)).await?;
    let running_depth: i64 = conn.zcard(running_key(queue)).await?;
    let failed_depth: i64 = conn.zcard(delayed_key(queue)).await?;
    let dead_letter_depth: i64 = conn.scard(dead_letter_key(queue)).await?;

    let mut statuses = Vec::new();
    if pending_depth > 0 {
        let oldest_id: Option<String> = conn.lindex(pending_key(queue), 0).await?;
        let oldest_age_seconds = if let Some(id) = oldest_id {
            if let Ok(job_id) = Uuid::parse_str(&id) {
                load_job(pool, queue, job_id)
                    .await
                    .ok()
                    .and_then(|job| job_age_seconds(&job, STATUS_PENDING))
            } else {
                None
            }
        } else {
            None
        };
        statuses.push(QueueStatusEntry {
            status: STATUS_PENDING.to_string(),
            depth: pending_depth,
            oldest_age_seconds,
        });
    }

    if running_depth > 0 {
        let oldest: Vec<String> = redis::cmd("ZRANGE")
            .arg(running_key(queue))
            .arg(0)
            .arg(0)
            .arg("WITHSCORES")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();
        let oldest_age_seconds = oldest
            .get(1)
            .and_then(|score| score.parse::<f64>().ok())
            .map(|score| {
                let age = now_ts() as f64 - score;
                age.max(0.0)
            });
        statuses.push(QueueStatusEntry {
            status: STATUS_RUNNING.to_string(),
            depth: running_depth,
            oldest_age_seconds,
        });
    }

    if failed_depth > 0 {
        let oldest: Vec<String> = redis::cmd("ZRANGE")
            .arg(delayed_key(queue))
            .arg(0)
            .arg(0)
            .arg("WITHSCORES")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();
        let oldest_age_seconds = oldest
            .get(1)
            .and_then(|score| score.parse::<f64>().ok())
            .map(|score| {
                let age = now_ts() as f64 - score;
                age.max(0.0)
            });
        statuses.push(QueueStatusEntry {
            status: STATUS_FAILED.to_string(),
            depth: failed_depth,
            oldest_age_seconds,
        });
    }

    if dead_letter_depth > 0 {
        statuses.push(QueueStatusEntry {
            status: STATUS_DEAD_LETTER.to_string(),
            depth: dead_letter_depth,
            oldest_age_seconds: None,
        });
    }

    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    async fn find_latest_job_returns_most_recent_match() {
        let redis = test_redis_pool().await;
        let queue = format!("worker_queue_test_{}", Uuid::new_v4());

        let older_job_id = enqueue_job(
            &redis,
            &queue,
            "email.send",
            serde_json::json!({
                "to_email": "old@example.test",
                "business_type": "verification",
                "html_body": "<a href=\"https://example.test/verify-result?token=old_token\">",
                "text_body": "verify-result?token=old_token",
            }),
            None,
            3,
            false,
            None,
        )
        .await
        .expect("older job");

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let newer_job_id = enqueue_job(
            &redis,
            &queue,
            "email.send",
            serde_json::json!({
                "to_email": "new@example.test",
                "business_type": "verification",
                "html_body": "<a href=\"https://example.test/verify-result?token=new_token\">",
                "text_body": "verify-result?token=new_token",
            }),
            None,
            3,
            false,
            None,
        )
        .await
        .expect("newer job");

        let job = find_latest_job(&redis, &queue, |job| {
            job.job_type == "email.send"
                && job
                    .payload
                    .get("business_type")
                    .and_then(serde_json::Value::as_str)
                    == Some("verification")
                && job
                    .payload
                    .get("to_email")
                    .and_then(serde_json::Value::as_str)
                    == Some("new@example.test")
        })
        .await
        .expect("job lookup")
        .expect("matching job");

        assert_eq!(job.id, newer_job_id);
        assert_ne!(job.id, older_job_id);
    }
}
