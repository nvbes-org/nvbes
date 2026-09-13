use redis::AsyncCommands;

use crate::connection::{RedisError, RedisPool};

pub async fn acquire(
    pool: &RedisPool,
    resource: &str,
    ttl_seconds: u64,
) -> Result<bool, RedisError> {
    let key = format!("nvbes:lock:{resource}");
    let mut conn = pool.get().await?;
    let acquired: bool = conn.set_nx(&key, "1").await?;
    if acquired {
        let _: bool = conn.expire(&key, ttl_seconds as i64).await?;
    }
    Ok(acquired)
}

pub async fn release(pool: &RedisPool, resource: &str) -> Result<(), RedisError> {
    let key = format!("nvbes:lock:{resource}");
    let mut conn = pool.get().await?;
    let _: () = conn.del(&key).await?;
    Ok(())
}

pub async fn with_lock<T, F, Fut>(
    pool: &RedisPool,
    resource: &str,
    ttl_seconds: u64,
    f: F,
) -> Result<Option<T>, RedisError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, RedisError>>,
{
    if !acquire(pool, resource, ttl_seconds).await? {
        return Ok(None);
    }

    let result = f().await;
    release(pool, resource).await?;
    result.map(Some)
}
