use redis::{AsyncCommands, Script};

use crate::connection::{RedisError, RedisPool};

use super::{QueuedJob, keys::*, retention::*};

const BATCH_SIZE: usize = 100;

const PROMOTE_SCRIPT: &str = r#"
local score = redis.call("ZSCORE", KEYS[1], ARGV[1])
if not score
    or tonumber(score) > tonumber(ARGV[4])
    or redis.call("GET", KEYS[3]) ~= ARGV[2] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[4])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[4])
end
redis.call("SET", KEYS[3], ARGV[3])
redis.call("ZREM", KEYS[1], ARGV[1])
redis.call("LREM", KEYS[2], 0, ARGV[1])
redis.call("RPUSH", KEYS[2], ARGV[1])
redis.call("ZREM", KEYS[4], ARGV[1])
redis.call("DEL", KEYS[5])
return 1
"#;

const CLEANUP_SCRIPT: &str = r#"
local score = redis.call("ZSCORE", KEYS[1], ARGV[1])
if not score or tonumber(score) > tonumber(ARGV[4]) then
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
redis.call("ZREM", KEYS[1], ARGV[1])
if ARGV[2] == "corrupt" then
    redis.call("DEL", KEYS[2])
    redis.call("ZREMRANGEBYSCORE", KEYS[3], "-inf", ARGV[7])
    redis.call("ZADD", KEYS[3], ARGV[6], ARGV[1])
    redis.call("EXPIRE", KEYS[3], ARGV[5])
end
return 1
"#;

const REPAIR_SCORE_SCRIPT: &str = r#"
local score = redis.call("ZSCORE", KEYS[1], ARGV[1])
if not score
    or tonumber(score) > tonumber(ARGV[4])
    or redis.call("GET", KEYS[2]) ~= ARGV[2] then
    return 0
end
redis.call("ZADD", KEYS[1], ARGV[3], ARGV[1])
return 1
"#;

pub async fn promote_due_jobs(pool: &RedisPool, queue: &str) -> Result<u64, RedisError> {
    let mut total = 0_u64;
    loop {
        let now = now_ts();
        let candidates = due_job_ids(pool, queue, now).await?;
        for id in &candidates {
            total += promote_job(pool, queue, id, now).await?;
        }
        if candidates.len() < BATCH_SIZE {
            return Ok(total);
        }
    }
}

async fn due_job_ids(pool: &RedisPool, queue: &str, now: i64) -> Result<Vec<String>, RedisError> {
    let mut conn = pool.get().await?;
    Ok(redis::cmd("ZRANGEBYSCORE")
        .arg(delayed_key(queue))
        .arg("-inf")
        .arg(now)
        .arg("LIMIT")
        .arg(0)
        .arg(BATCH_SIZE)
        .query_async(&mut *conn)
        .await?)
}

async fn promote_job(pool: &RedisPool, queue: &str, id: &str, now: i64) -> Result<u64, RedisError> {
    let current_raw = load_raw(pool, queue, id).await?;
    let Some(current_raw) = current_raw else {
        cleanup_delayed_pointer(pool, queue, id, None, "missing", now).await?;
        return Ok(0);
    };
    let Ok(mut job) = serde_json::from_str::<QueuedJob>(&current_raw) else {
        cleanup_delayed_pointer(pool, queue, id, Some(&current_raw), "corrupt", now).await?;
        return Ok(0);
    };
    if job.queue != queue || job.id.to_string() != id {
        cleanup_delayed_pointer(pool, queue, id, Some(&current_raw), "corrupt", now).await?;
        return Ok(0);
    }
    if job.status != STATUS_FAILED {
        cleanup_delayed_pointer(pool, queue, id, Some(&current_raw), "stale", now).await?;
        return Ok(0);
    }
    if job.available_at > now {
        repair_delayed_score(pool, queue, id, &current_raw, job.available_at, now).await?;
        return Ok(0);
    }
    job.status = STATUS_PENDING.to_string();
    job.available_at = now;
    job.updated_at = now;
    let updated_raw = serde_json::to_string(&job)?;
    let mut conn = pool.get().await?;
    Ok(Script::new(PROMOTE_SCRIPT)
        .key(delayed_key(queue))
        .key(pending_key(queue))
        .key(job_key(queue, job.id))
        .key(dead_letter_key(queue))
        .key(lease_key(queue, job.id))
        .arg(id)
        .arg(current_raw)
        .arg(updated_raw)
        .arg(now)
        .invoke_async(&mut *conn)
        .await?)
}

async fn load_raw(pool: &RedisPool, queue: &str, id: &str) -> Result<Option<String>, RedisError> {
    let mut conn = pool.get().await?;
    Ok(conn.get(queue_key(queue, &format!("job:{id}"))).await?)
}

async fn cleanup_delayed_pointer(
    pool: &RedisPool,
    queue: &str,
    id: &str,
    raw: Option<&str>,
    state: &str,
    now: i64,
) -> Result<(), RedisError> {
    let current_time = now_ts();
    let retention_seconds = terminal_retention_seconds();
    let expires_at = terminal_expiry(current_time, retention_seconds);
    let mut conn = pool.get().await?;
    let _: u64 = Script::new(CLEANUP_SCRIPT)
        .key(delayed_key(queue))
        .key(queue_key(queue, &format!("job:{id}")))
        .key(dead_letter_key(queue))
        .arg(id)
        .arg(state)
        .arg(raw.unwrap_or_default())
        .arg(now)
        .arg(retention_seconds)
        .arg(expires_at)
        .arg(current_time)
        .invoke_async(&mut *conn)
        .await?;
    Ok(())
}

async fn repair_delayed_score(
    pool: &RedisPool,
    queue: &str,
    id: &str,
    raw: &str,
    available_at: i64,
    now: i64,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: u64 = Script::new(REPAIR_SCORE_SCRIPT)
        .key(delayed_key(queue))
        .key(queue_key(queue, &format!("job:{id}")))
        .arg(id)
        .arg(raw)
        .arg(available_at)
        .arg(now)
        .invoke_async(&mut *conn)
        .await?;
    Ok(())
}
