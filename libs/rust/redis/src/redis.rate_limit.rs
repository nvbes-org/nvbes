use redis::AsyncCommands;
use redis::Script;

use crate::connection::{RedisError, RedisPool, command_with_timeout};

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub current: u64,
    pub limit: u64,
    pub ttl_seconds: u64,
}

const RATE_LIMIT_LUA: &str = r#"
local current = redis.call('INCR', KEYS[1])
if current == 1 then
    redis.call('EXPIRE', KEYS[1], ARGV[1])
end
local ttl = redis.call('TTL', KEYS[1])
if ttl < 0 then
    ttl = tonumber(ARGV[1])
end
return {current, ttl}
"#;

pub async fn check_rate_limit(
    pool: &RedisPool,
    bucket: &str,
    action: &str,
    key: &str,
    max_hits: u64,
    window_seconds: u64,
) -> Result<RateLimitResult, RedisError> {
    let redis_key = format!("nvbes:rl:{bucket}:{action}:{key}");
    let mut conn = pool.get().await?;

    let script = Script::new(RATE_LIMIT_LUA);
    let result: (i64, i64) = command_with_timeout(
        "rate_limit_check",
        script
            .key(&redis_key)
            .arg(window_seconds)
            .invoke_async(&mut *conn),
    )
    .await?;

    let current = result.0 as u64;
    let ttl = result.1.max(0) as u64;

    Ok(RateLimitResult {
        allowed: current <= max_hits,
        current,
        limit: max_hits,
        ttl_seconds: ttl,
    })
}

pub async fn reset_rate_limit(
    pool: &RedisPool,
    bucket: &str,
    action: &str,
    key: &str,
) -> Result<(), RedisError> {
    let redis_key = format!("nvbes:rl:{bucket}:{action}:{key}");
    let mut conn = pool.get().await?;
    let _: () = command_with_timeout("rate_limit_reset", conn.del(&redis_key)).await?;
    Ok(())
}

pub async fn get_rate_limit_status(
    pool: &RedisPool,
    bucket: &str,
    action: &str,
    key: &str,
    max_hits: u64,
) -> Result<RateLimitResult, RedisError> {
    let redis_key = format!("nvbes:rl:{bucket}:{action}:{key}");
    let mut conn = pool.get().await?;

    let current: Option<i64> =
        command_with_timeout("rate_limit_status_get", conn.get(&redis_key)).await?;
    let ttl: i64 = command_with_timeout("rate_limit_status_ttl", conn.ttl(&redis_key)).await?;

    let current = current.unwrap_or(0) as u64;
    let ttl = ttl.max(0) as u64;

    Ok(RateLimitResult {
        allowed: current <= max_hits,
        current,
        limit: max_hits,
        ttl_seconds: ttl,
    })
}
