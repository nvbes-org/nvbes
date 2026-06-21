use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    routing::{get, post},
};
use serde::Serialize;

use super::csp_report;
use super::middleware::{csrf, dpop, idempotency, origin};
use super::observability;
use super::openapi;
use super::well_known;
use crate::app::AppState;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: &crate::app::AppState) -> Router<crate::app::AppState> {
    const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

    let private_observability_routes = Router::new()
        .route(
            "/metrics",
            axum::routing::get(nvbes_observability::metrics_handler),
        )
        .route("/observability/dashboards", get(dashboards))
        .route("/observability/alerts/critical", get(critical_alerts))
        .route("/observability/log-streams", get(log_streams))
        .route(
            "/observability/error-reporting-smoke",
            post(error_reporting_smoke),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::internal_observability::internal_observability_guard,
        ));

    let public_report_routes = Router::new()
        .route("/observability/network-errors", post(network_error_reports))
        .route("/csp-report", post(csp_report::csp_report_handler))
        .layer(axum::middleware::from_fn(
            |req: Request<Body>, next: Next| async {
                let mut res = next.run(req).await;
                nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 3600, 86400);
                res
            },
        ));

    let docs = openapi::openapi_routes().layer(axum::middleware::from_fn(
        |req: Request<Body>, next: Next| async {
            let mut res = next.run(req).await;
            nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 86400, 0);
            res
        },
    ));

    axum::Router::new()
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_SIZE))
        .route("/health", get(health))
        .route("/.well-known/dpop-nonce", get(dpop_nonce_handler))
        .route(
            "/.well-known/change-password",
            get(well_known::change_password_well_known),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(crate::domains::oauth::metadata::oauth_authorization_server_metadata),
        )
        .route(
            "/.well-known/openid-configuration",
            get(crate::domains::oauth::metadata::openid_configuration),
        )
        .route("/.well-known/gpc.json", get(well_known::gpc_well_known))
        .route(
            "/.well-known/passkey-endpoints",
            get(well_known::passkey_endpoints_well_known),
        )
        .route(
            "/.well-known/webauthn",
            get(well_known::webauthn_well_known),
        )
        .route(
            "/.well-known/security.txt",
            get(well_known::security_txt_well_known),
        )
        .merge(private_observability_routes)
        .merge(public_report_routes)
        .merge(crate::domains::router(state))
        .nest("/oauth", crate::domains::oauth::routes::router(state))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            dpop::dpop_auth_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            origin::origin_guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            csrf::csrf_guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            idempotency::idempotency_guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::e2ee::request_e2ee_guard,
        ))
        .layer(axum::middleware::from_fn(
            nvbes_core::http::content_digest::content_digest_guard,
        ))
        .merge(crate::email::webhooks::webhook_router(state))
        .merge(docs)
}

async fn health(State(state): State<crate::app::AppState>) -> Json<HealthResponse> {
    state.observability.record_postgres_pool(
        &state.config.app_name,
        &state.config.environment,
        state.db.size(),
        state.db.num_idle(),
    );

    Json(HealthResponse { status: "ok" })
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
    State(state): State<AppState>,
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

async fn dpop_nonce_handler(State(state): State<crate::app::AppState>) -> axum::response::Response {
    let nonce = state
        .dpop_nonce
        .as_ref()
        .map(|d| async move { d.generate().await.unwrap_or_default() });
    let nonce = match nonce {
        Some(nonce) => nonce.await,
        None => String::new(),
    };
    axum::response::Response::builder()
        .header(dpop::D_POP_NONCE_HEADER, nonce)
        .status(axum::http::StatusCode::OK)
        .body(axum::body::Body::empty())
        .unwrap()
}
