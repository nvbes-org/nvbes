use crate::connection::{RedisError, RedisPool};
use redis::AsyncCommands;
use serde::Serialize;
use serde::de::DeserializeOwned;

type RedisResult<T> = Result<T, RedisError>;

#[derive(Clone)]
pub struct RedisClient {
    pool: RedisPool,
}

impl RedisClient {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &RedisPool {
        &self.pool
    }

    pub async fn get(&self, key: &str) -> RedisResult<Option<String>> {
        let mut conn = self.pool.get().await?;
        Ok(conn.get(key).await?)
    }

    pub async fn set(&self, key: &str, value: &str) -> RedisResult<()> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.set(key, value).await?;
        Ok(())
    }

    pub async fn set_with_ttl(&self, key: &str, value: &str, ttl_seconds: u64) -> RedisResult<()> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.set_ex(key, value, ttl_seconds).await?;
        Ok(())
    }

    pub async fn del(&self, keys: &[&str]) -> RedisResult<()> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.del(keys).await?;
        Ok(())
    }

    pub async fn del_key(&self, key: &str) -> RedisResult<()> {
        self.del(&[key]).await
    }

    pub async fn exists(&self, key: &str) -> RedisResult<bool> {
        let mut conn = self.pool.get().await?;
        let result: bool = conn.exists(key).await?;
        Ok(result)
    }

    pub async fn expire(&self, key: &str, ttl_seconds: i64) -> RedisResult<()> {
        let mut conn = self.pool.get().await?;
        let _: bool = conn.expire(key, ttl_seconds).await?;
        Ok(())
    }

    pub async fn ttl(&self, key: &str) -> RedisResult<i64> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.ttl(key).await?;
        Ok(result)
    }

    pub async fn incr(&self, key: &str) -> RedisResult<i64> {
        let mut conn = self.pool.get().await?;
        let result: i64 = conn.incr(key, 1).await?;
        Ok(result)
    }

    pub async fn cache_get_json<T: DeserializeOwned>(&self, key: &str) -> RedisResult<Option<T>> {
        match self.get(key).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn cache_set_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: u64,
    ) -> RedisResult<()> {
        let json = serde_json::to_string(value)?;
        self.set_with_ttl(key, &json, ttl_seconds).await
    }

    pub async fn sadd(&self, key: &str, member: &str) -> RedisResult<bool> {
        let mut conn = self.pool.get().await?;
        let result: bool = conn.sadd(key, member).await?;
        Ok(result)
    }

    pub async fn sismember(&self, key: &str, member: &str) -> RedisResult<bool> {
        let mut conn = self.pool.get().await?;
        let result: bool = conn.sismember(key, member).await?;
        Ok(result)
    }

    pub async fn smembers(&self, key: &str) -> RedisResult<Vec<String>> {
        let mut conn = self.pool.get().await?;
        let result: Vec<String> = conn.smembers(key).await?;
        Ok(result)
    }

    pub async fn srem(&self, key: &str, member: &str) -> RedisResult<bool> {
        let mut conn = self.pool.get().await?;
        let result: bool = conn.srem(key, member).await?;
        Ok(result)
    }

    pub async fn hget(&self, key: &str, field: &str) -> RedisResult<Option<String>> {
        let mut conn = self.pool.get().await?;
        Ok(conn.hget(key, field).await?)
    }

    pub async fn hset(&self, key: &str, field: &str, value: &str) -> RedisResult<()> {
        let mut conn = self.pool.get().await?;
        let _: bool = conn.hset(key, field, value).await?;
        Ok(())
    }

    pub async fn hgetall(&self, key: &str) -> RedisResult<Vec<(String, String)>> {
        let mut conn = self.pool.get().await?;
        let result: Vec<(String, String)> = conn.hgetall(key).await?;
        Ok(result)
    }

    pub async fn hdel(&self, key: &str, fields: &[&str]) -> RedisResult<()> {
        let mut conn = self.pool.get().await?;
        let _: bool = conn.hdel(key, fields).await?;
        Ok(())
    }

    pub async fn publish(&self, channel: &str, message: &str) -> RedisResult<i32> {
        let mut conn = self.pool.get().await?;
        let result: i32 = conn.publish(channel, message).await?;
        Ok(result)
    }
}
