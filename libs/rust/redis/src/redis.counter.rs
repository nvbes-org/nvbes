use redis::AsyncCommands;

use crate::connection::{RedisError, RedisPool};

pub async fn incr(pool: &RedisPool, key: &str) -> Result<i64, RedisError> {
    let mut conn = pool.get().await?;
    let value: i64 = conn.incr(key, 1).await?;
    Ok(value)
}

pub async fn incr_with_ttl(
    pool: &RedisPool,
    key: &str,
    ttl_seconds: u64,
) -> Result<i64, RedisError> {
    let mut conn = pool.get().await?;
    let value: i64 = conn.incr(key, 1).await?;
    if value == 1 {
        let _: bool = conn.expire(key, ttl_seconds as i64).await?;
    }
    Ok(value)
}

pub async fn decr(pool: &RedisPool, key: &str) -> Result<i64, RedisError> {
    let mut conn = pool.get().await?;
    let value: i64 = conn.decr(key, 1).await?;
    Ok(value)
}

pub async fn get_counter(pool: &RedisPool, key: &str) -> Result<i64, RedisError> {
    let mut conn = pool.get().await?;
    let value: Option<i64> = conn.get(key).await?;
    Ok(value.unwrap_or(0))
}

pub async fn reset_counter(pool: &RedisPool, key: &str) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: () = conn.del(key).await?;
    Ok(())
}
