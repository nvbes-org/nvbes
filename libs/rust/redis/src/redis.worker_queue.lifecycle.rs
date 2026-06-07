use std::time::Duration;

use redis::AsyncCommands;
use serde_json::Value;
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

use super::{
    QueuedJob,
    keys::*,
    store::{load_job, move_to_pending, remove_from_indexes, save_job},
};

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
