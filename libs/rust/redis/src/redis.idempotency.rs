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

pub async fn delete_if_value(
    pool: &RedisPool,
    key: &str,
    expected_value: &str,
) -> Result<bool, RedisError> {
    let mut conn = pool.get().await?;
    let deleted: i32 = redis::Script::new(
        r#"
        if redis.call("GET", KEYS[1]) == ARGV[1] then
            return redis.call("DEL", KEYS[1])
        end
        return 0
        "#,
    )
    .key(key)
    .arg(expected_value)
    .invoke_async(&mut *conn)
    .await?;
    Ok(deleted > 0)
}
