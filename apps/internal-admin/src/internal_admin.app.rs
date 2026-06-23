use axum::{Router, http::StatusCode};
use std::time::Duration;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;

use nvbes_core::config::AppConfig;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub rate_limiter: crate::rate_limit::BackofficeRateLimiter,
}

impl AppState {
    pub fn new(config: AppConfig, db: PgPool) -> Self {
        Self {
            config,
            db,
            rate_limiter: crate::rate_limit::BackofficeRateLimiter::default(),
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    crate::routes::router(&state.config)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::rate_limit::backoffice_rate_limit,
        ))
        .layer(axum::middleware::from_fn(crate::routes::security_headers))
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
