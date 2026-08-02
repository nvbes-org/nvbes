use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    routing::{get, post},
};
use serde::Serialize;

use super::account_closure;
use super::observability;
use super::openapi;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HealthResponse {
    release_id: &'static str,
    status: &'static str,
}

pub fn router(state: &crate::app::AppState) -> Router<crate::app::AppState> {
    let private_observability_routes = Router::new()
        .route("/metrics", get(nvbes_observability::metrics_handler))
        .route("/observability/dashboards", get(dashboards))
        .route("/observability/alerts/critical", get(critical_alerts))
        .route("/observability/log-streams", get(log_streams))
        .route(
            "/observability/error-reporting-smoke",
            post(error_reporting_smoke),
        )
        .layer(middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::internal_observability::internal_observability_guard,
        ));

    let public_report_routes = Router::new()
        .route("/observability/network-errors", post(network_error_reports))
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async {
                let mut res = next.run(req).await;
                nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 3600, 86400);
                res
            },
        ));

    let docs = openapi::openapi_routes().layer(middleware::from_fn(
        |req: Request<Body>, next: Next| async {
            let mut res = next.run(req).await;
            nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 86400, 0);
            res
        },
    ));

    Router::new()
        .route("/health", get(health))
        .merge(account_closure::router())
        .merge(private_observability_routes)
        .merge(public_report_routes)
        .merge(docs)
}

async fn health(State(state): State<crate::app::AppState>) -> Json<HealthResponse> {
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

async fn dashboards() -> Json<observability::DashboardsResponse> {
    Json(observability::DashboardsResponse::v1())
}

async fn critical_alerts() -> Json<observability::CriticalAlertsResponse> {
    Json(observability::CriticalAlertsResponse::v1())
}

async fn log_streams() -> Json<observability::LogStreamsResponse> {
    Json(observability::LogStreamsResponse::v1())
}

async fn error_reporting_smoke(
    State(state): State<crate::app::AppState>,
) -> Json<nvbes_observability::ErrorReportingSmokeResult> {
    Json(nvbes_observability::capture_error_reporting_smoke(
        &state.config.app_name,
        &state.config.environment,
        "api",
        false,
    ))
}

async fn network_error_reports() -> StatusCode {
    StatusCode::NO_CONTENT
}
