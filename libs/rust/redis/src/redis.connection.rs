use bb8_redis::{RedisConnectionManager, bb8};
use tracing::info;

use crate::config::RedisConfig;

pub type RedisPool = bb8::Pool<RedisConnectionManager>;

#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    #[error("Redis connection error: {0}")]
    Connection(String),
    #[error("Redis command error: {0}")]
    Command(#[from] redis::RedisError),
    #[error("Redis pool error: {0}")]
    Pool(#[from] bb8::RunError<redis::RedisError>),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn create_pool(config: &RedisConfig) -> Result<RedisPool, RedisError> {
    let mut redis_url = config.url.clone();
    if let (Some(password), Some(pos)) = (config.password.as_ref(), redis_url.find("://")) {
        let prefix = &redis_url[..pos + 3];
        let rest = &redis_url[pos + 3..];
        redis_url = format!("{prefix}:{password}@{rest}");
    }

    let manager = RedisConnectionManager::new(redis_url.as_str())
        .map_err(|e| RedisError::Connection(e.to_string()))?;

    let pool = bb8::Pool::builder()
        .max_size(config.max_connections)
        .min_idle(Some(1))
        .connection_timeout(std::time::Duration::from_secs(5))
        .build(manager)
        .await
        .map_err(|e| RedisError::Connection(e.to_string()))?;

    info!(
        url = %config.url,
        max_connections = config.max_connections,
        "Redis connection pool initialized"
    );

    Ok(pool)
}

pub async fn health_check(pool: &RedisPool) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: String = redis::cmd("PING").query_async(&mut *conn).await?;
    Ok(())
}
