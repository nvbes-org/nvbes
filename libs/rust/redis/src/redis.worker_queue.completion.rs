use std::time::Duration;

use redis::Script;
use serde_json::Value;

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*, retention::*};

const COMPLETE_SCRIPT: &str = r#"
local current_raw = redis.call("GET", KEYS[1])
if not current_raw
    or current_raw ~= ARGV[1]
    or redis.call("GET", KEYS[6]) ~= ARGV[6] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[5])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[5])
end
redis.call("SET", KEYS[1], ARGV[2])
redis.call("LREM", KEYS[2], 0, ARGV[3])
redis.call("ZREM", KEYS[3], ARGV[3])
redis.call("ZREM", KEYS[4], ARGV[3])
redis.call("ZREM", KEYS[5], ARGV[3])
redis.call("DEL", KEYS[6])
if ARGV[4] == "retry" then
    redis.call("ZADD", KEYS[4], ARGV[5], ARGV[3])
else
    redis.call("EXPIRE", KEYS[1], ARGV[7])
    if ARGV[9] == "1" and redis.call("GET", KEYS[7]) == ARGV[3] then
        redis.call("EXPIRE", KEYS[7], ARGV[7])
    end
    redis.call("ZREMRANGEBYSCORE", KEYS[5], "-inf", ARGV[10])
    if ARGV[4] == "dead_letter" then
        redis.call("ZADD", KEYS[5], ARGV[8], ARGV[3])
        redis.call("EXPIRE", KEYS[5], ARGV[7])
    end
end
return 1
"#;

pub async fn mark_job_succeeded(
    pool: &RedisPool,
    job: &QueuedJob,
    _result: Value,
) -> Result<(), RedisError> {
    mark_job_succeeded_with_retention(pool, job, terminal_retention_seconds()).await
}

pub(super) async fn mark_job_succeeded_with_retention(
    pool: &RedisPool,
    job: &QueuedJob,
    retention_seconds: u64,
) -> Result<(), RedisError> {
    let now = now_ts();
    let finished = succeeded_snapshot(job, now);
    transition_claimed_job(pool, job, &finished, "succeeded", now, retention_seconds).await
}

pub async fn mark_job_failed(
    pool: &RedisPool,
    job: &QueuedJob,
    error: &str,
    retryable: bool,
    retry_delay: Duration,
) -> Result<(), RedisError> {
    mark_job_failed_with_retention(
        pool,
        job,
        error,
        retryable,
        retry_delay,
        terminal_retention_seconds(),
    )
    .await
}

pub(super) async fn mark_job_failed_with_retention(
    pool: &RedisPool,
    job: &QueuedJob,
    error: &str,
    retryable: bool,
    retry_delay: Duration,
    retention_seconds: u64,
) -> Result<(), RedisError> {
    let now = now_ts();
    let mut failed = job.clone();
    failed.updated_at = now;
    failed.last_error = Some(error.to_string());
    failed.claimed_at = None;
    failed.lease_token = None;
    failed.result = None;

    let outcome = if retryable && failed.attempts < failed.max_attempts {
        failed.status = STATUS_FAILED.to_string();
        failed.available_at = now + retry_delay.as_secs() as i64;
        "retry"
    } else {
        failed = dead_letter_snapshot(job, now);
        "dead_letter"
    };
    transition_claimed_job(pool, job, &failed, outcome, now, retention_seconds).await
}

async fn transition_claimed_job(
    pool: &RedisPool,
    claimed: &QueuedJob,
    updated: &QueuedJob,
    outcome: &str,
    now: i64,
    retention_seconds: u64,
) -> Result<(), RedisError> {
    let raw = serde_json::to_string(updated)?;
    let claimed_raw = serde_json::to_string(claimed)?;
    let expires_at = terminal_expiry(now, retention_seconds);
    let lease_token = claimed
        .lease_token
        .ok_or_else(|| RedisError::JobLeaseLost {
            queue: claimed.queue.clone(),
            job_id: claimed.id,
        })?;
    let id = claimed.id.to_string();
    let dedupe = claimed
        .idempotency_key
        .as_deref()
        .map(|key| dedupe_key(&claimed.queue, &claimed.job_type, key));
    let unused_dedupe_key = queue_key(&claimed.queue, &format!("unused-dedupe:{}", claimed.id));
    let mut conn = pool.get().await?;
    let changed: i64 = Script::new(COMPLETE_SCRIPT)
        .key(job_key(&claimed.queue, claimed.id))
        .key(pending_key(&claimed.queue))
        .key(running_key(&claimed.queue))
        .key(delayed_key(&claimed.queue))
        .key(dead_letter_key(&claimed.queue))
        .key(lease_key(&claimed.queue, claimed.id))
        .key(dedupe.as_deref().unwrap_or(&unused_dedupe_key))
        .arg(claimed_raw)
        .arg(raw)
        .arg(&id)
        .arg(outcome)
        .arg(updated.available_at)
        .arg(lease_token.to_string())
        .arg(retention_seconds)
        .arg(expires_at)
        .arg(if dedupe.is_some() { "1" } else { "0" })
        .arg(now)
        .invoke_async(&mut *conn)
        .await?;
    if changed == 1 {
        Ok(())
    } else {
        Err(RedisError::JobLeaseLost {
            queue: claimed.queue.clone(),
            job_id: claimed.id,
        })
    }
}
