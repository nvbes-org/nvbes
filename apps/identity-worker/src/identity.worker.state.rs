use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis: nvbes_redis::RedisPool,
    pub email: nvbes_email::EmailClient,
    pub enterprise_grpc: Option<crate::worker::enterprise_grpc::EnterpriseGrpcConfig>,
    pub inline_housekeeping_enabled: bool,
    pub observability: nvbes_observability::metrics::HttpMetrics,
    pub account_projection: crate::worker::account_projection::client::AccountProjectionClient,
}

impl AppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        let redis = nvbes_core::redis_runtime::require_redis_pool(config).await?;
        let email = nvbes_email::EmailClient::connect(nvbes_email::EmailClientConfig::from_env(
            &config.environment,
        )?)
        .await?;
        let enterprise_grpc =
            crate::worker::enterprise_grpc::EnterpriseGrpcConfig::from_env(&config.environment)?;
        let inline_housekeeping_enabled = inline_housekeeping_enabled()?;
        let observability = nvbes_observability::metrics::HttpMetrics::default();
        let account_projection =
            crate::worker::account_projection::client::AccountProjectionClient::from_env(
                &config.environment,
            )?;

        observability.record_postgres_pool(
            "identity-worker",
            &config.environment,
            db.size(),
            db.num_idle(),
        );

        Ok(Self {
            config: config.clone(),
            db,
            redis,
            email,
            enterprise_grpc,
            inline_housekeeping_enabled,
            observability,
            account_projection,
        })
    }
}

fn inline_housekeeping_enabled() -> anyhow::Result<bool> {
    std::env::var("NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED")
        .map(|value| value.parse::<bool>())
        .unwrap_or(Ok(true))
        .map_err(|_| {
            anyhow::anyhow!(
                "NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED must be true or false"
            )
        })
}
