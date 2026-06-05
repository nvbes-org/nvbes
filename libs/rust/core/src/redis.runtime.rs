use crate::config::AppConfig;

pub async fn require_redis_pool(
    config: &AppConfig,
) -> Result<nvbes_redis::RedisPool, nvbes_redis::RedisError> {
    let redis_config = nvbes_redis::RedisConfig {
        url: config.redis_url.clone(),
        password: config.redis_password.clone(),
        max_connections: config.redis_max_connections,
    };

    let pool = nvbes_redis::connection::create_pool(&redis_config).await?;
    nvbes_redis::connection::health_check(&pool).await?;
    tracing::info!("Redis: connected and healthy");
    Ok(pool)
}
