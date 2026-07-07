use axum::{
    Json, Router,
    extract::State,
    http::Request,
    middleware::Next,
    routing::{get, post},
};
use serde::Serialize;

use crate::app::BillingAppState;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HealthResponse {
    release_id: &'static str,
    status: &'static str,
}

pub fn router(state: &BillingAppState) -> Router<BillingAppState> {
    let observability_routes = Router::new()
        .route(
            "/metrics",
            axum::routing::get(nvbes_observability::metrics_handler),
        )
        .route(
            "/observability/error-reporting-smoke",
            post(error_reporting_smoke),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::internal_observability::internal_observability_guard,
        ));

    let internal_billing_routes =
        crate::domains::usage::router().layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::internal_observability::internal_observability_guard,
        ));

    Router::new()
        .route("/health", get(health))
        .merge(observability_routes)
        .merge(crate::domains::public_workspace::router())
        .merge(crate::domains::webhooks::router())
        .merge(internal_billing_routes)
        .layer(axum::middleware::from_fn(
            |req: Request<axum::body::Body>, next: Next| async move {
                let mut res = next.run(req).await;
                nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 0, 0);
                res
            },
        ))
}

async fn health(State(state): State<BillingAppState>) -> Json<HealthResponse> {
    state.observability.record_postgres_pool(
        &state.config.app_name,
        &state.config.environment,
        state.db.size(),
        state.db.num_idle(),
    );

    Json(HealthResponse {
        release_id: env!("CARGO_PKG_VERSION"),
        status: "ok",
    })
}

async fn error_reporting_smoke(
    State(state): State<BillingAppState>,
) -> Json<nvbes_observability::ErrorReportingSmokeResult> {
    Json(nvbes_observability::capture_error_reporting_smoke(
        "billing-service",
        &state.config.environment,
        "api",
        state.config.sentry_dsn.is_some(),
    ))
}
