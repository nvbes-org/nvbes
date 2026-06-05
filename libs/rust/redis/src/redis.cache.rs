use redis::AsyncCommands;
use redis::Script;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::connection::{RedisError, RedisPool};

pub async fn cache_get(pool: &RedisPool, key: &str) -> Result<Option<String>, RedisError> {
    let mut conn = pool.get().await?;
    Ok(conn.get(key).await?)
}

pub async fn cache_set(
    pool: &RedisPool,
    key: &str,
    value: &str,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: () = conn.set_ex(key, value, ttl_seconds).await?;
    Ok(())
}

pub async fn cache_del(pool: &RedisPool, key: &str) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: () = conn.del(key).await?;
    Ok(())
}

pub async fn cache_take(pool: &RedisPool, key: &str) -> Result<Option<String>, RedisError> {
    let mut conn = pool.get().await?;
    let script = Script::new(
        r#"
local value = redis.call('GET', KEYS[1])
if value then
    redis.call('DEL', KEYS[1])
end
return value
"#,
    );
    let value: Option<String> = script.key(key).invoke_async(&mut *conn).await?;
    Ok(value)
}

pub async fn cache_get_json<T: DeserializeOwned>(
    pool: &RedisPool,
    key: &str,
) -> Result<Option<T>, RedisError> {
    match cache_get(pool, key).await? {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

pub async fn cache_set_json<T: Serialize>(
    pool: &RedisPool,
    key: &str,
    value: &T,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let json = serde_json::to_string(value)?;
    cache_set(pool, key, &json, ttl_seconds).await
}

pub async fn cache_get_or_set<T, F, Fut>(
    pool: &RedisPool,
    key: &str,
    ttl_seconds: u64,
    fallback: F,
) -> Result<T, RedisError>
where
    T: DeserializeOwned + Serialize + Clone,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, RedisError>>,
{
    if let Some(cached) = cache_get_json::<T>(pool, key).await? {
        return Ok(cached);
    }

    let value = fallback().await?;
    cache_set_json(pool, key, &value, ttl_seconds).await?;
    Ok(value)
}
