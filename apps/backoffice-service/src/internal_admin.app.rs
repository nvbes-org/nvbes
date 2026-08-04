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
    pub(crate) privileged_identity: crate::privileged_authentication::PrivilegedIdentityClient,
    pub(crate) email_operations:
        std::sync::Arc<dyn crate::email_operations::BackofficeEmailOperations>,
    pub billing_grpc_endpoint: String,
    pub observability: nvbes_observability::metrics::HttpMetrics,
    pub rate_limiter: crate::rate_limit::BackofficeRateLimiter,
}

impl axum::extract::FromRef<AppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

impl AppState {
    pub fn new(config: AppConfig, db: PgPool) -> Self {
        #[cfg(not(test))]
        {
            Self::try_new(config, db).expect("backoffice service configuration must be valid")
        }
        #[cfg(test)]
        {
            let billing_grpc_endpoint = crate::billing_grpc::billing_grpc_endpoint(config.api_port)
                .unwrap_or_else(|_| "http://127.0.0.1:3021".to_string());
            Self {
                privileged_identity:
                    crate::privileged_authentication::PrivilegedIdentityClient::from_environment(
                        &config.environment,
                    )
                    .expect("test identity configuration must be valid"),
                email_operations: crate::email_operations::test_gateway(),
                observability: nvbes_observability::metrics::HttpMetrics::default(),
                rate_limiter: crate::rate_limit::BackofficeRateLimiter::default(),
                billing_grpc_endpoint,
                config,
                db,
            }
        }
    }

    pub fn try_new(config: AppConfig, db: PgPool) -> Result<Self, String> {
        let billing_grpc_endpoint = crate::billing_grpc::billing_grpc_endpoint(config.api_port)
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "falling back to local Billing gRPC endpoint");
                "http://127.0.0.1:3021".to_string()
            });
        let privileged_identity =
            crate::privileged_authentication::PrivilegedIdentityClient::from_environment(
                &config.environment,
            )?;
        let email_operations = crate::email_operations::from_environment(&config.environment)?;
        let state = Self {
            config,
            db,
            privileged_identity,
            email_operations,
            billing_grpc_endpoint,
            observability: nvbes_observability::metrics::HttpMetrics::default(),
            rate_limiter: crate::rate_limit::BackofficeRateLimiter::default(),
        };
        state.observability.record_postgres_pool(
            &state.config.app_name,
            &state.config.environment,
            state.db.size(),
            state.db.num_idle(),
        );
        Ok(state)
    }
}

pub fn build_router(state: AppState) -> Router {
    crate::routes::router(&state)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::rate_limit::backoffice_rate_limit,
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
