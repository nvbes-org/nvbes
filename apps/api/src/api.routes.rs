use axum::{Router, routing::get};
use nvbes_core::http::internal_observability::{
    InternalObservabilityConfig, internal_observability_guard_with_config,
};
use nvbes_observability::metrics::HttpMetrics;
use nvbes_observability::middleware::observe_request;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};

use crate::{config::ApiConfig, health, identity, openapi};

pub fn build_router(pool: PgPool, config: &ApiConfig) -> Router {
    let http_metrics = HttpMetrics::new();
    let consent_service = Arc::new(nvbes_consent::service::ConsentService::new(
        pool.clone(),
        "nvbes_consent_ip_salt",
    ));
    let consent_router = Router::new().nest(
        "/api/v1/consents",
        nvbes_consent::routes::consent_routes(consent_service),
    );

    let (identity_router, _token_service) = identity::build_router(pool.clone(), config);

    let observability_config = InternalObservabilityConfig::new(
        &config.environment,
        config.observability_internal_token.as_deref(),
    );
    let metrics_router = Router::new()
        .route("/metrics", get(nvbes_observability::metrics_handler))
        .layer(axum::middleware::from_fn_with_state(
            observability_config,
            internal_observability_guard_with_config,
        ))
        .with_state(http_metrics.clone());

    let mut router = Router::new()
        .merge(health::router().with_state(pool.clone()))
        .merge(metrics_router)
        .merge(openapi::router())
        .merge(consent_router)
        .merge(identity_router);

    // Integrate Account domain if configuration is present
    if let Ok(account_config) = nvbes_account_service::AccountConfig::from_env() {
        match nvbes_account_service::AccountState::new(account_config, pool.clone()) {
            Ok(account_state) => {
                tracing::info!("mounting Account service domain router");
                router = router.merge(nvbes_account_service::domain_router(account_state));
            }
            Err(e) => {
                tracing::warn!("failed to initialize Account state: {e}");
            }
        }
    } else {
        tracing::info!("Account configuration not set, skipping Account router mounting");
    }

    // Integrate Billing domain if configuration is present
    if let Ok(billing_config) = nvbes_billing_service::BillingConfig::from_env() {
        match nvbes_billing_service::auth::TokenVerifier::new(&billing_config) {
            Ok(tokens) => {
                let billing_state = nvbes_billing_service::BillingState {
                    db: pool.clone(),
                    config: billing_config,
                    metrics: http_metrics.handle.clone(),
                    tokens,
                    email_client: None,
                };
                tracing::info!("mounting Billing service domain router");
                router = router.merge(nvbes_billing_service::domain_router(billing_state));
            }
            Err(e) => {
                tracing::warn!("failed to initialize Billing TokenVerifier: {e}");
            }
        }
    } else {
        tracing::info!("Billing configuration not set, skipping Billing router mounting");
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    router
        .layer(axum::middleware::from_fn_with_state(
            http_metrics,
            observe_request,
        ))
        .layer(CatchPanicLayer::new())
        .layer(cors)
}
