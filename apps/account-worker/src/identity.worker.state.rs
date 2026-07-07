use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis: nvbes_redis::RedisPool,
    pub email: Arc<dyn nvbes_email::EmailSender>,
    pub observability: nvbes_observability::metrics::HttpMetrics,
}

impl AppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        let redis = nvbes_core::redis_runtime::require_redis_pool(config).await?;
        let email = nvbes_product_account::email::delivery::build_email_sender(config)?;
        let observability = nvbes_observability::metrics::HttpMetrics::default();

        observability.record_postgres_pool(
            &config.app_name,
            &config.environment,
            db.size(),
            db.num_idle(),
        );

        Ok(Self {
            config: config.clone(),
            db,
            redis,
            email,
            observability,
        })
    }
}
