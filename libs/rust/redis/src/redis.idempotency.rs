use redis::AsyncCommands;

use crate::connection::{RedisError, RedisPool};

pub async fn check_and_set(
    pool: &RedisPool,
    key: &str,
    response_hash: &str,
    ttl_seconds: u64,
) -> Result<(bool, Option<String>), RedisError> {
    let mut conn = pool.get().await?;

    let existing: Option<String> = conn.get(key).await?;

    if let Some(existing_hash) = existing {
        return Ok((false, Some(existing_hash)));
    }

    let result: bool = conn.set_nx(key, response_hash).await?;
    if result {
        let _: bool = conn.expire(key, ttl_seconds as i64).await?;
        Ok((true, None))
    } else {
        let existing: Option<String> = conn.get(key).await?;
        Ok((false, existing))
    }
}

pub async fn get_response(pool: &RedisPool, key: &str) -> Result<Option<String>, RedisError> {
    let mut conn = pool.get().await?;
    Ok(conn.get(key).await?)
}

pub async fn delete(pool: &RedisPool, key: &str) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: () = conn.del(key).await?;
    Ok(())
}
