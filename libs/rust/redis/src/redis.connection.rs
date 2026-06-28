use bb8_redis::{RedisConnectionManager, bb8};
use std::future::Future;
use std::time::Duration;
use tracing::{info, warn};

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
    #[error("Redis operation timed out: {0}")]
    OperationTimeout(&'static str),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl RedisError {
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Connection(_) => true,
            Self::Command(error) => redis_error_is_transient(error),
            Self::Pool(bb8::RunError::TimedOut) => true,
            Self::Pool(bb8::RunError::User(error)) => redis_error_is_transient(error),
            Self::OperationTimeout(_) => true,
            Self::Json(_) => false,
        }
    }
}

fn redis_error_is_transient(error: &redis::RedisError) -> bool {
    error.is_unrecoverable_error()
        || error.is_timeout()
        || error.is_connection_refusal()
        || error.is_connection_dropped()
        || matches!(
            error.kind(),
            redis::ErrorKind::BusyLoadingError
                | redis::ErrorKind::TryAgain
                | redis::ErrorKind::ClusterDown
        )
}

#[derive(Debug, Clone, Copy)]
struct RedisPoolErrorSink;

impl bb8::ErrorSink<redis::RedisError> for RedisPoolErrorSink {
    fn sink(&self, error: redis::RedisError) {
        let transient = redis_error_is_transient(&error);
        warn!(%error, transient, "Redis pool background error");
    }

    fn boxed_clone(&self) -> Box<dyn bb8::ErrorSink<redis::RedisError>> {
        Box::new(*self)
    }
}

pub async fn create_pool(config: &RedisConfig) -> Result<RedisPool, RedisError> {
    let redis_url = redis_url_with_password(config);

    let manager = RedisConnectionManager::new(redis_url.as_str())
        .map_err(|e| RedisError::Connection(e.to_string()))?;

    let pool = bb8::Pool::builder()
        .max_size(config.max_connections)
        .min_idle(Some(1))
        .test_on_check_out(true)
        .connection_timeout(Duration::from_secs(5))
        .idle_timeout(Some(Duration::from_secs(5 * 60)))
        .max_lifetime(Some(Duration::from_secs(30 * 60)))
        .retry_connection(true)
        .error_sink(Box::new(RedisPoolErrorSink))
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

fn redis_url_with_password(config: &RedisConfig) -> String {
    let Some(password) = config.password.as_ref() else {
        return config.url.clone();
    };
    let Some(scheme_end) = config.url.find("://") else {
        return config.url.clone();
    };

    let prefix_end = scheme_end + 3;
    let rest = &config.url[prefix_end..];
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.contains('@') {
        return config.url.clone();
    }

    format!("{}:{}@{}", &config.url[..prefix_end], password, rest)
}

pub async fn health_check(pool: &RedisPool) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: String = command_with_timeout(
        "redis_health_ping",
        redis::cmd("PING").query_async(&mut *conn),
    )
    .await?;
    Ok(())
}

pub async fn command_with_timeout<T, F>(operation: &'static str, future: F) -> Result<T, RedisError>
where
    F: Future<Output = redis::RedisResult<T>>,
{
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .map_err(|_| RedisError::OperationTimeout(operation))?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{RedisConfig, RedisError, redis_url_with_password};

    #[test]
    fn redis_error_is_transient_for_dropped_connections() {
        let error = redis::RedisError::from(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "connection reset by peer",
        ));

        assert!(RedisError::Command(error).is_transient());
    }

    #[test]
    fn redis_error_is_not_transient_for_json_decode_failures() {
        let error = serde_json::from_str::<serde_json::Value>("not-json").unwrap_err();

        assert!(!RedisError::Json(error).is_transient());
    }

    #[test]
    fn redis_url_with_password_injects_password_when_url_has_no_credentials() {
        let config = RedisConfig {
            url: "redis://redis:6379".to_string(),
            password: Some("redis_dev".to_string()),
            max_connections: 10,
        };

        assert_eq!(
            redis_url_with_password(&config),
            "redis://:redis_dev@redis:6379"
        );
    }

    #[test]
    fn redis_url_with_password_preserves_url_credentials() {
        let config = RedisConfig {
            url: "redis://:redis_dev@redis:6379".to_string(),
            password: Some("redis_dev".to_string()),
            max_connections: 10,
        };

        assert_eq!(
            redis_url_with_password(&config),
            "redis://:redis_dev@redis:6379"
        );
    }
}
