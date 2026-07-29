use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use redis::{AsyncCommands, Script};

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*, promotion::promote_due_jobs, retention::*};

const EMPTY_POLL_INTERVAL: Duration = Duration::from_millis(100);
const PROMOTION_INTERVAL: Duration = Duration::from_secs(1);
static NEXT_QUEUE: AtomicUsize = AtomicUsize::new(0);

const CLAIM_SCRIPT: &str = r#"
if redis.call("LINDEX", KEYS[1], 0) ~= ARGV[1]
    or redis.call("GET", KEYS[2]) ~= ARGV[2] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[5])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[5])
end
redis.call("LPOP", KEYS[1])
redis.call("LREM", KEYS[1], 0, ARGV[1])
redis.call("SET", KEYS[2], ARGV[3])
redis.call("ZADD", KEYS[3], ARGV[4], ARGV[1])
redis.call("ZREM", KEYS[4], ARGV[1])
redis.call("ZREM", KEYS[5], ARGV[1])
redis.call("SET", KEYS[6], ARGV[5])
return 1
"#;

const DISCARD_POINTER_SCRIPT: &str = r#"
if redis.call("LINDEX", KEYS[1], 0) ~= ARGV[1] then
    return 0
end
local current = redis.call("GET", KEYS[2])
if ARGV[2] == "missing" then
    if current then return 0 end
elseif current ~= ARGV[3] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[3])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[3])
end
redis.call("LPOP", KEYS[1])
redis.call("LREM", KEYS[1], 0, ARGV[1])
if ARGV[2] ~= "stale" then
    redis.call("DEL", KEYS[4])
end
if ARGV[2] == "corrupt" then
    redis.call("DEL", KEYS[2])
    redis.call("ZREMRANGEBYSCORE", KEYS[3], "-inf", ARGV[6])
    redis.call("ZADD", KEYS[3], ARGV[5], ARGV[1])
    redis.call("EXPIRE", KEYS[3], ARGV[4])
end
return 1
"#;

pub async fn claim_next_job(
    pool: &RedisPool,
    queues: &[&str],
    timeout_seconds: u64,
) -> Result<Option<QueuedJob>, RedisError> {
    if queues.is_empty() {
        return Ok(None);
    }
    let start = tokio::time::Instant::now();
    let timeout = (timeout_seconds > 0).then(|| Duration::from_secs(timeout_seconds));
    let first_queue = NEXT_QUEUE.fetch_add(1, Ordering::Relaxed) % queues.len();
    let mut next_promotion = start;
    loop {
        let should_promote = tokio::time::Instant::now() >= next_promotion;
        for offset in 0..queues.len() {
            let queue = queues[(first_queue + offset) % queues.len()];
            if should_promote {
                promote_due_jobs(pool, queue).await?;
            }
            if let Some(job) = claim_from_queue(pool, queue).await? {
                return Ok(Some(job));
            }
        }
        if should_promote {
            next_promotion = tokio::time::Instant::now() + PROMOTION_INTERVAL;
        }
        if timeout.is_some_and(|limit| start.elapsed() >= limit) {
            return Ok(None);
        }
        tokio::time::sleep(EMPTY_POLL_INTERVAL).await;
    }
}

async fn claim_from_queue(pool: &RedisPool, queue: &str) -> Result<Option<QueuedJob>, RedisError> {
    loop {
        let Some((id, raw)) = peek_pending_job(pool, queue).await? else {
            return Ok(None);
        };
        let Ok(mut job) = serde_json::from_str::<QueuedJob>(&raw) else {
            discard_pending_pointer(pool, queue, &id, Some(&raw), true).await?;
            continue;
        };
        if job.status != STATUS_PENDING || job.queue != queue || job.id.to_string() != id {
            discard_pending_pointer(pool, queue, &id, Some(&raw), false).await?;
            continue;
        }
        let now = now_ts();
        job.status = STATUS_RUNNING.to_string();
        job.attempts = job.attempts.saturating_add(1);
        job.claimed_at = Some(now);
        let lease_token = uuid::Uuid::new_v4();
        job.lease_token = Some(lease_token);
        job.updated_at = now;
        job.last_error = None;
        let updated_raw = serde_json::to_string(&job)?;
        let mut conn = pool.get().await?;
        let claimed: u64 = Script::new(CLAIM_SCRIPT)
            .key(pending_key(queue))
            .key(job_key(queue, job.id))
            .key(running_key(queue))
            .key(delayed_key(queue))
            .key(dead_letter_key(queue))
            .key(lease_key(queue, job.id))
            .arg(&id)
            .arg(&raw)
            .arg(updated_raw)
            .arg(now)
            .arg(lease_token.to_string())
            .invoke_async(&mut *conn)
            .await?;
        if claimed == 1 {
            return Ok(Some(job));
        }
    }
}

async fn peek_pending_job(
    pool: &RedisPool,
    queue: &str,
) -> Result<Option<(String, String)>, RedisError> {
    let mut conn = pool.get().await?;
    let id: Option<String> = conn.lindex(pending_key(queue), 0).await?;
    let Some(id) = id else {
        return Ok(None);
    };
    let raw: Option<String> = conn.get(queue_key(queue, &format!("job:{id}"))).await?;
    match raw {
        Some(raw) => Ok(Some((id, raw))),
        None => {
            discard_pending_pointer(pool, queue, &id, None, false).await?;
            Ok(None)
        }
    }
}

async fn discard_pending_pointer(
    pool: &RedisPool,
    queue: &str,
    id: &str,
    raw: Option<&str>,
    corrupt: bool,
) -> Result<(), RedisError> {
    let state = if raw.is_none() {
        "missing"
    } else if corrupt {
        "corrupt"
    } else {
        "stale"
    };
    let now = now_ts();
    let retention_seconds = terminal_retention_seconds();
    let expires_at = terminal_expiry(now, retention_seconds);
    let mut conn = pool.get().await?;
    let _: u64 = Script::new(DISCARD_POINTER_SCRIPT)
        .key(pending_key(queue))
        .key(queue_key(queue, &format!("job:{id}")))
        .key(dead_letter_key(queue))
        .key(queue_key(queue, &format!("lease:{id}")))
        .arg(id)
        .arg(state)
        .arg(raw.unwrap_or_default())
        .arg(retention_seconds)
        .arg(expires_at)
        .arg(now)
        .invoke_async(&mut *conn)
        .await?;
    Ok(())
}
