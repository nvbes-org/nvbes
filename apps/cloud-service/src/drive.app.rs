use axum::http::StatusCode;
use axum::{Router, middleware};
use nvbes_core::config::AppConfig;
use nvbes_scan::ScanEngine;
use nvbes_storage::ObjectStore;
use std::time::Duration;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;

use crate::{db::Database, domains, http};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: Database,
    pub observability: nvbes_observability::metrics::HttpMetrics,
    pub product_analytics: nvbes_product_analytics::ProductAnalytics,
    pub storage: std::sync::Arc<dyn ObjectStore>,
    pub scanner: std::sync::Arc<dyn ScanEngine>,
    pub redis: nvbes_redis::RedisPool,
    pub rate_limiter: nvbes_core::limiter::RateLimiter,
    pub internal_service_token: String,
}

impl axum::extract::FromRef<AppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

pub async fn build_storage(config: &AppConfig) -> std::sync::Arc<dyn ObjectStore> {
    nvbes_product_cloud::storage::build_storage(config).await
}

fn build_scanner(config: &AppConfig) -> std::sync::Arc<dyn ScanEngine> {
    if !config.scan_enabled {
        tracing::warn!(
            "Scan: disabled by configuration; uploaded objects will be marked unscanned_disabled"
        );
        return std::sync::Arc::new(nvbes_scan::MockScanner::new());
    }

    match config.scan_engine.as_str() {
        "clamav" => {
            tracing::info!(
                host = %config.clamav_host,
                port = config.clamav_port,
                timeout = config.scan_timeout_secs,
                "Scan: ClamAV"
            );
            std::sync::Arc::new(nvbes_scan::ClamAvScanner::new(
                config.clamav_host.clone(),
                config.clamav_port,
                config.scan_timeout_secs,
            ))
        }
        other => panic!(
            "Unsupported scan engine '{other}'. Use scan_engine=clamav in non-development environments."
        ),
    }
}

pub async fn build_app_state(config: AppConfig, db: Database) -> anyhow::Result<AppState> {
    if config.environment != "development" && config.scan_fail_open {
        panic!("SCAN_FAIL_OPEN is forbidden outside development.");
    }
    ensure_separate_untrusted_content_origin(&config)?;

    let storage = build_storage(&config).await;
    let scanner = build_scanner(&config);

    let redis = nvbes_core::redis_runtime::require_redis_pool(&config).await?;

    let rate_limiter = nvbes_core::limiter::RateLimiter::new(redis.clone());
    let internal_service_token = nvbes_core::http::internal_service::load_token(
        "NVBES_CLOUD_INTERNAL_TOKEN",
        &config.environment,
        "development-cloud-internal-token-01",
    )
    .map_err(anyhow::Error::msg)?;

    let product_analytics = build_product_analytics(&config)?;
    let state = AppState {
        config,
        db,
        observability: nvbes_observability::metrics::HttpMetrics::default(),
        product_analytics,
        storage,
        scanner,
        redis,
        rate_limiter,
        internal_service_token,
    };

    state.observability.record_postgres_pool(
        &state.config.app_name,
        &state.config.environment,
        state.db.size(),
        state.db.num_idle(),
    );

    Ok(state)
}

fn ensure_separate_untrusted_content_origin(config: &AppConfig) -> anyhow::Result<()> {
    if config.environment == "development" {
        return Ok(());
    }

    let Some(storage_endpoint) = config
        .storage_public_endpoint
        .as_deref()
        .or(config.storage_endpoint.as_deref())
    else {
        return Ok(());
    };
    let web_origin = reqwest::Url::parse(&config.web_base_url)?.origin();
    let storage_origin = reqwest::Url::parse(storage_endpoint)?.origin();
    if web_origin == storage_origin {
        anyhow::bail!(
            "STORAGE_PUBLIC_ENDPOINT must use an origin distinct from NVBES_WEB_BASE_URL outside development"
        );
    }
    Ok(())
}

pub fn build_router(state: AppState) -> Router {
    const MAX_BODY_SIZE: usize = 64 * 1024 * 1024;

    let cors = nvbes_core::security::cors_layer(&state.config);
    let http_request_timeout_secs = state.config.http_request_timeout_secs;
    let api_max_connections_per_ip = state.config.api_max_connections_per_ip;
    let api_max_concurrent_requests = state.config.api_max_concurrent_requests;

    Router::new()
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(CompressionLayer::new())
        .merge(http::router(&state))
        .merge(domains::router(&state))
        .layer(middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::e2ee::request_e2ee_guard,
        ))
        .layer(middleware::from_fn(
            nvbes_core::http::content_digest::content_digest_guard,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            nvbes_observability::middleware::observe_request,
        ))
        .layer(middleware::from_fn(http::security_headers))
        .layer(middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::client_ip::trusted_client_ip_middleware,
        ))
        .layer(
            nvbes_core::http::connection_limit::PerIpConcurrencyLayer::new(
                api_max_connections_per_ip,
            ),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(http_request_timeout_secs),
        ))
        .layer(cors)
        .layer(ConcurrencyLimitLayer::new(
            api_max_concurrent_requests as usize,
        ))
        .with_state(state)
}

#[cfg(test)]
pub async fn build_app(config: AppConfig, db: Database) -> anyhow::Result<Router> {
    Ok(build_router(build_app_state(config, db).await?))
}

fn build_product_analytics(
    config: &AppConfig,
) -> anyhow::Result<nvbes_product_analytics::ProductAnalytics> {
    let analytics_config = nvbes_product_analytics::ProductAnalyticsConfig {
        enabled: config.product_analytics_enabled,
        analytics_id_salt: config.analytics_id_salt.clone(),
    };

    if !config.product_analytics_enabled {
        tracing::info!("Cloud product analytics disabled");
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
    tracing::info!("Cloud product analytics enabled with PostHog");

    Ok(nvbes_product_analytics::ProductAnalytics::with_sink(
        analytics_config,
        std::sync::Arc::new(sink),
    )?)
}

#[cfg(test)]
mod security_tests {
    use super::ensure_separate_untrusted_content_origin;
    use nvbes_core::config::AppConfig;

    #[test]
    fn production_rejects_storage_on_the_web_origin() {
        let config = AppConfig {
            environment: "production".to_owned(),
            web_base_url: "https://cloud.nvbes.fr".to_owned(),
            storage_endpoint: Some("https://cloud.nvbes.fr/s3".to_owned()),
            ..AppConfig::default()
        };

        assert!(ensure_separate_untrusted_content_origin(&config).is_err());
    }

    #[test]
    fn production_accepts_a_dedicated_content_origin() {
        let config = AppConfig {
            environment: "production".to_owned(),
            web_base_url: "https://cloud.nvbes.fr".to_owned(),
            storage_endpoint: Some("https://content.nvbes.fr".to_owned()),
            ..AppConfig::default()
        };

        assert!(ensure_separate_untrusted_content_origin(&config).is_ok());
    }

    #[test]
    fn production_validates_the_public_storage_origin() {
        let config = AppConfig {
            environment: "production".to_owned(),
            web_base_url: "https://cloud.nvbes.fr".to_owned(),
            storage_endpoint: Some("http://storage.internal".to_owned()),
            storage_public_endpoint: Some("https://cloud.nvbes.fr/s3".to_owned()),
            ..AppConfig::default()
        };

        assert!(ensure_separate_untrusted_content_origin(&config).is_err());
    }
}
