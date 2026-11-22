use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::{config::ApiConfig, health, identity, openapi};

pub fn build_router(pool: PgPool, config: &ApiConfig) -> Router {
    let consent_service = Arc::new(nvbes_consent::service::ConsentService::new(
        pool.clone(),
        "nvbes_consent_ip_salt",
    ));
    let consent_router = Router::new().nest(
        "/api/v1/consents",
        nvbes_consent::routes::consent_routes(consent_service),
    );

    let (identity_router, _token_service) = identity::build_router(pool.clone(), config);

    let mut router = Router::new()
        .merge(health::router().with_state(pool.clone()))
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
        let metrics = metrics_exporter_prometheus::PrometheusBuilder::new()
            .build_recorder()
            .handle();
        match nvbes_billing_service::auth::TokenVerifier::new(&billing_config) {
            Ok(tokens) => {
                let billing_state = nvbes_billing_service::BillingState {
                    db: pool.clone(),
                    config: billing_config,
                    metrics: Arc::new(metrics),
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

    router.layer(cors).layer(TraceLayer::new_for_http())
}
