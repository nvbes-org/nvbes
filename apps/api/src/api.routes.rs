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

use crate::{
    client_ip::{TrustedProxies, resolve_client_ip},
    config::ApiConfig,
    health, identity,
    identity_activity::InProcessActivity,
    identity_runtime::IdentityRuntime,
    openapi,
};

pub fn build_router(
    pool: PgPool,
    config: &ApiConfig,
    identity: &IdentityRuntime,
) -> anyhow::Result<Router> {
    let http_metrics = HttpMetrics::new();
    let identity_router = identity::build_router(pool.clone(), identity)?;
    let account_state = nvbes_account_service::AccountState::new(
        config.account.clone(),
        identity.trust.clone(),
        pool.clone(),
        Arc::new(InProcessActivity::new(
            pool.clone(),
            identity.clients.clone(),
            identity.tokens.clone(),
        )),
    )?;

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

    let router = Router::new()
        .merge(health::router().with_state(pool.clone()))
        .merge(metrics_router)
        .merge(openapi::router())
        .merge(identity_router)
        .merge(nvbes_account_service::domain_router(account_state));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Ok(router
        .layer(axum::middleware::from_fn_with_state(
            TrustedProxies::new(&config.trusted_proxy_cidrs),
            resolve_client_ip,
        ))
        .layer(axum::middleware::from_fn_with_state(
            http_metrics,
            observe_request,
        ))
        .layer(CatchPanicLayer::new())
        .layer(cors))
}
