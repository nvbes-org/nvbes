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
}

impl axum::extract::FromRef<AppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

pub async fn build_storage(config: &AppConfig) -> std::sync::Arc<dyn ObjectStore> {
    if config.storage_enabled {
        let endpoint = config
            .storage_endpoint
            .as_deref()
            .expect("STORAGE_ENDPOINT required when STORAGE_ENABLED is true");
        let access_key = config
            .storage_access_key
            .as_deref()
            .expect("STORAGE_ACCESS_KEY required when STORAGE_ENABLED is true");
        let secret_key = config
            .storage_secret_key
            .as_deref()
            .expect("STORAGE_SECRET_KEY required when STORAGE_ENABLED is true");

        tracing::info!(
            bucket = %config.storage_bucket,
            endpoint,
            region = %config.storage_region,
            "Storage: S3 (real)"
        );

        std::sync::Arc::new(
            nvbes_storage::S3ObjectStore::new(
                config.storage_bucket.clone(),
                endpoint,
                &config.storage_region,
                access_key,
                secret_key,
            )
            .await,
        )
    } else {
        if config.environment != "development" {
            panic!(
                "Mock storage is forbidden outside development. Enable STORAGE_ENABLED and configure S3-compatible credentials."
            );
        }
        tracing::info!("Storage: Mock (development mode)");
        std::sync::Arc::new(nvbes_storage::MockObjectStore::new())
    }
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

pub async fn build_app(config: AppConfig, db: Database) -> anyhow::Result<Router> {
    if config.environment != "development" && config.scan_fail_open {
        panic!("SCAN_FAIL_OPEN is forbidden outside development.");
    }

    let storage = build_storage(&config).await;
    let scanner = build_scanner(&config);

    let redis = nvbes_core::redis_runtime::require_redis_pool(&config).await?;

    let rate_limiter = nvbes_core::limiter::RateLimiter::new(redis.clone());

    let cors = nvbes_core::security::cors_layer(&config);
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
    };

    state.observability.record_postgres_pool(
        &state.config.app_name,
        &state.config.environment,
        state.db.size(),
        state.db.num_idle(),
    );

    const MAX_BODY_SIZE: usize = 64 * 1024 * 1024;

    Ok(Router::new()
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
                state.config.api_max_connections_per_ip,
            ),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.http_request_timeout_secs),
        ))
        .layer(cors)
        .layer(ConcurrencyLimitLayer::new(
            state.config.api_max_concurrent_requests as usize,
        ))
        .with_state(state))
}

fn build_product_analytics(
    config: &AppConfig,
) -> anyhow::Result<nvbes_product_analytics::ProductAnalytics> {
    Ok(nvbes_product_analytics::ProductAnalytics::new(
        nvbes_product_analytics::ProductAnalyticsConfig {
            enabled: config.product_analytics_enabled,
            analytics_id_salt: config.analytics_id_salt.clone(),
        },
    )?)
}
