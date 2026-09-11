use nvbes_redis::{RedisConfig, RedisPool, connection};

const DEFAULT_MAX_CONNECTIONS: u32 = 2;
const CONNECT_TIMEOUT_MS: u64 = 500;

/// Creates a Redis pool suitable for integration tests.
///
/// Unifies the two variants that existed across crates:
/// - Rewrites `localhost` to `127.0.0.1` for Docker compatibility
/// - Limits connections to 2 to avoid exhausting test Redis
/// - Applies a short connect timeout to fail fast when Redis is unavailable
/// - Performs a health check before returning the pool
///
/// Returns `None` if Redis is unreachable. Required integration tests must
/// fail explicitly on `None`; infrastructure absence is never a passing test.
pub async fn test_redis_pool() -> Option<RedisPool> {
    test_redis_pool_with_max_connections(DEFAULT_MAX_CONNECTIONS).await
}

/// Like [`test_redis_pool`] but with a custom max connection limit.
pub async fn test_redis_pool_with_max_connections(max_connections: u32) -> Option<RedisPool> {
    let mut config = RedisConfig::from_env();
    if config.url == "redis://localhost:6379" {
        config.url = "redis://127.0.0.1:6379".to_string();
    }
    config.max_connections = max_connections;
    let pool = tokio::time::timeout(
        std::time::Duration::from_millis(CONNECT_TIMEOUT_MS),
        connection::create_pool(&config),
    )
    .await
    .ok()
    .and_then(|r| r.ok())?;

    if connection::health_check(&pool).await.is_err() {
        return None;
    }

    Some(pool)
}
