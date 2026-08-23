use axum::http::StatusCode;
pub use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use std::time::Duration;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;

#[derive(Clone)]
pub struct BillingAppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis: nvbes_redis::RedisPool,
    pub rate_limiter: nvbes_core::limiter::RateLimiter,
    pub observability: nvbes_observability::metrics::HttpMetrics,
    pub product_analytics: nvbes_product_analytics::ProductAnalytics,
    pub internal_service_token: String,
}

impl axum::extract::FromRef<BillingAppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &BillingAppState) -> Self {
        state.observability.clone()
    }
}

impl BillingAppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        let redis = nvbes_core::redis_runtime::require_redis_pool(config).await?;
        let internal_service_token = nvbes_core::http::internal_service::load_token(
            "NVBES_BILLING_INTERNAL_TOKEN",
            &config.environment,
            "development-billing-internal-token",
        )
        .map_err(anyhow::Error::msg)?;
        Ok(Self {
            config: config.clone(),
            db,
            redis: redis.clone(),
            rate_limiter: nvbes_core::limiter::RateLimiter::new(redis),
            observability: nvbes_observability::metrics::HttpMetrics::default(),
            product_analytics: build_product_analytics(config)?,
            internal_service_token,
        })
    }
}

fn build_product_analytics(
    config: &AppConfig,
) -> anyhow::Result<nvbes_product_analytics::ProductAnalytics> {
    if !config.product_analytics_enabled {
        tracing::info!("Billing product analytics disabled");
        return Ok(nvbes_product_analytics::ProductAnalytics::disabled());
    }

    let sink = nvbes_analytics_posthog::PostHogAnalyticsSink::new(
        nvbes_analytics_posthog::PostHogAnalyticsConfig {
            host: config.posthog_host.clone(),
            project_token: config.product_analytics_token.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "NVBES_PRODUCT_ANALYTICS_TOKEN is required when product analytics is enabled"
                )
            })?,
        },
    )?;
    tracing::info!("Billing product analytics enabled with PostHog");

    Ok(nvbes_product_analytics::ProductAnalytics::with_sink(
        nvbes_product_analytics::ProductAnalyticsConfig {
            enabled: true,
            analytics_id_salt: config.analytics_id_salt.clone(),
        },
        std::sync::Arc::new(sink),
    )?)
}

pub fn build_router(state: BillingAppState) -> axum::Router {
    crate::http::router(&state)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            nvbes_observability::middleware::observe_request,
        ))
        .layer(axum::middleware::from_fn(
            nvbes_core::security::security_headers,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::client_ip::trusted_client_ip_middleware,
        ))
        .layer(
            nvbes_core::http::connection_limit::PerIpConcurrencyLayer::new(
                state.config.api_max_connections_per_ip,
            ),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.http_request_timeout_secs),
        ))
        .layer(ConcurrencyLimitLayer::new(
            state.config.api_max_concurrent_requests as usize,
        ))
        .with_state(state)
}
