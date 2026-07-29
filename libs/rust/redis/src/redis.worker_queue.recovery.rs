use std::time::Duration;

use redis::{AsyncCommands, Script};

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*, retention::*};

#[path = "redis.worker_queue.recovery.cleanup.rs"]
mod cleanup;
use cleanup::{CLEANUP_SCRIPT, REPAIR_SCORE_SCRIPT};

const BATCH_SIZE: usize = 100;

struct RecoveryTransition<'a> {
    queue: &'a str,
    id: &'a str,
    current_raw: String,
    job: QueuedJob,
    now: i64,
    cutoff: i64,
    retry_delay: Duration,
    invalid_lease: bool,
}

const RECOVER_SCRIPT: &str = r#"
local score = redis.call("ZSCORE", KEYS[1], ARGV[1])
if not score
    or tonumber(score) > tonumber(ARGV[6])
    or redis.call("GET", KEYS[5]) ~= ARGV[2] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[4])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[4])
end
redis.call("SET", KEYS[5], ARGV[3])
redis.call("ZREM", KEYS[1], ARGV[1])
redis.call("LREM", KEYS[2], 0, ARGV[1])
redis.call("ZREM", KEYS[3], ARGV[1])
redis.call("ZREM", KEYS[4], ARGV[1])
redis.call("DEL", KEYS[6])
if ARGV[4] == "failed" then
    redis.call("ZADD", KEYS[3], ARGV[5], ARGV[1])
else
    redis.call("EXPIRE", KEYS[5], ARGV[7])
    if ARGV[9] == "1" and redis.call("GET", KEYS[7]) == ARGV[1] then
        redis.call("EXPIRE", KEYS[7], ARGV[7])
    end
    redis.call("ZREMRANGEBYSCORE", KEYS[4], "-inf", ARGV[10])
    redis.call("ZADD", KEYS[4], ARGV[8], ARGV[1])
    redis.call("EXPIRE", KEYS[4], ARGV[7])
end
return 1
"#;

pub async fn recover_stale_jobs(
    pool: &RedisPool,
    queue: &str,
    stale_after: Duration,
    retry_delay: Duration,
) -> Result<Vec<(String, String)>, RedisError> {
    let mut recovered = Vec::new();
    loop {
        let now = now_ts();
        let cutoff = now - stale_after.as_secs() as i64;
        let candidates = stale_job_ids(pool, queue, cutoff).await?;
        for id in &candidates {
            if let Some(outcome) = recover_job(pool, queue, id, now, cutoff, retry_delay).await? {
                recovered.push(outcome);
            }
        }
        if candidates.len() < BATCH_SIZE {
            return Ok(recovered);
        }
    }
}

async fn stale_job_ids(
    pool: &RedisPool,
    queue: &str,
    cutoff: i64,
) -> Result<Vec<String>, RedisError> {
    let mut conn = pool.get().await?;
    Ok(redis::cmd("ZRANGEBYSCORE")
        .arg(running_key(queue))
        .arg("-inf")
        .arg(cutoff)
        .arg("LIMIT")
        .arg(0)
        .arg(BATCH_SIZE)
        .query_async(&mut *conn)
        .await?)
}

async fn recover_job(
    pool: &RedisPool,
    queue: &str,
    id: &str,
    now: i64,
    cutoff: i64,
    retry_delay: Duration,
) -> Result<Option<(String, String)>, RedisError> {
    let current_raw = load_raw(pool, queue, id).await?;
    let Some(current_raw) = current_raw else {
        cleanup_running_pointer(pool, queue, id, None, "missing", cutoff).await?;
        return Ok(None);
    };
    let Ok(job) = serde_json::from_str::<QueuedJob>(&current_raw) else {
        let changed =
            cleanup_running_pointer(pool, queue, id, Some(&current_raw), "corrupt", cutoff).await?;
        return Ok(changed.then(|| ("corrupt".to_string(), STATUS_DEAD_LETTER.to_string())));
    };
    if job.queue != queue || job.id.to_string() != id {
        let changed =
            cleanup_running_pointer(pool, queue, id, Some(&current_raw), "corrupt", cutoff).await?;
        return Ok(changed.then(|| ("corrupt".to_string(), STATUS_DEAD_LETTER.to_string())));
    }
    if job.status != STATUS_RUNNING {
        cleanup_running_pointer(pool, queue, id, Some(&current_raw), "stale", cutoff).await?;
        return Ok(None);
    }
    let Some(claimed_at) = job.claimed_at else {
        return transition_recovered_job(
            pool,
            RecoveryTransition {
                queue,
                id,
                current_raw,
                job,
                now,
                cutoff,
                retry_delay,
                invalid_lease: true,
            },
        )
        .await;
    };
    if claimed_at > cutoff {
        repair_running_score(pool, queue, id, &current_raw, claimed_at, cutoff).await?;
        return Ok(None);
    }
    transition_recovered_job(
        pool,
        RecoveryTransition {
            queue,
            id,
            current_raw,
            job,
            now,
            cutoff,
            retry_delay,
            invalid_lease: false,
        },
    )
    .await
}

async fn transition_recovered_job(
    pool: &RedisPool,
    transition: RecoveryTransition<'_>,
) -> Result<Option<(String, String)>, RedisError> {
    let RecoveryTransition {
        queue,
        id,
        current_raw,
        mut job,
        now,
        cutoff,
        retry_delay,
        invalid_lease,
    } = transition;
    let dead_letter = invalid_lease || job.attempts >= job.max_attempts;
    let outcome = if dead_letter {
        STATUS_DEAD_LETTER
    } else {
        STATUS_FAILED
    };
    let job_type = job.job_type.clone();
    let dedupe = job
        .idempotency_key
        .as_deref()
        .map(|key| dedupe_key(queue, &job.job_type, key));
    let unused_dedupe_key = queue_key(queue, &format!("unused-dedupe:{id}"));
    if dead_letter {
        job = dead_letter_snapshot(&job, now);
    } else {
        job.status = outcome.to_string();
        job.available_at = now + retry_delay.as_secs() as i64;
        job.updated_at = now;
        job.last_error = Some(if invalid_lease {
            "job lease metadata is invalid".to_string()
        } else {
            "job lease expired".to_string()
        });
        job.claimed_at = None;
        job.lease_token = None;
        job.result = None;
    }
    let retention_seconds = terminal_retention_seconds();
    let expires_at = terminal_expiry(now, retention_seconds);
    let updated_raw = serde_json::to_string(&job)?;
    let mut conn = pool.get().await?;
    let changed: u64 = Script::new(RECOVER_SCRIPT)
        .key(running_key(queue))
        .key(pending_key(queue))
        .key(delayed_key(queue))
        .key(dead_letter_key(queue))
        .key(queue_key(queue, &format!("job:{id}")))
        .key(queue_key(queue, &format!("lease:{id}")))
        .key(dedupe.as_deref().unwrap_or(&unused_dedupe_key))
        .arg(id)
        .arg(current_raw)
        .arg(updated_raw)
        .arg(outcome)
        .arg(job.available_at)
        .arg(cutoff)
        .arg(retention_seconds)
        .arg(expires_at)
        .arg(if dedupe.is_some() { "1" } else { "0" })
        .arg(now)
        .invoke_async(&mut *conn)
        .await?;
    Ok((changed == 1).then(|| (job_type, outcome.to_string())))
}

async fn load_raw(pool: &RedisPool, queue: &str, id: &str) -> Result<Option<String>, RedisError> {
    let mut conn = pool.get().await?;
    Ok(conn.get(queue_key(queue, &format!("job:{id}"))).await?)
}

async fn cleanup_running_pointer(
    pool: &RedisPool,
    queue: &str,
    id: &str,
    raw: Option<&str>,
    state: &str,
    cutoff: i64,
) -> Result<bool, RedisError> {
    let now = now_ts();
    let retention_seconds = terminal_retention_seconds();
    let expires_at = terminal_expiry(now, retention_seconds);
    let mut conn = pool.get().await?;
    let changed: u64 = Script::new(CLEANUP_SCRIPT)
        .key(running_key(queue))
        .key(queue_key(queue, &format!("job:{id}")))
        .key(dead_letter_key(queue))
        .key(queue_key(queue, &format!("lease:{id}")))
        .arg(id)
        .arg(state)
        .arg(raw.unwrap_or_default())
        .arg(cutoff)
        .arg(retention_seconds)
        .arg(expires_at)
        .arg(now)
        .invoke_async(&mut *conn)
        .await?;
    Ok(changed == 1)
}

async fn repair_running_score(
    pool: &RedisPool,
    queue: &str,
    id: &str,
    raw: &str,
    claimed_at: i64,
    cutoff: i64,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: u64 = Script::new(REPAIR_SCORE_SCRIPT)
        .key(running_key(queue))
        .key(queue_key(queue, &format!("job:{id}")))
        .arg(id)
        .arg(raw)
        .arg(claimed_at)
        .arg(cutoff)
        .invoke_async(&mut *conn)
        .await?;
    Ok(())
}
