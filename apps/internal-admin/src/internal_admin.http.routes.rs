use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
    routing::get,
};
use serde::Serialize;

use crate::app::AppState;
use nvbes_core::config::AppConfig;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HealthResponse {
    release_id: &'static str,
    status: &'static str,
}

pub fn router(config: &AppConfig) -> Router<AppState> {
    let private_routes = Router::new()
        .merge(crate::access_center::router())
        .merge(crate::audit::router())
        .merge(crate::audit_evidence_center::router())
        .merge(crate::billing_admin::router())
        .merge(crate::billing_admin_overview::router())
        .merge(crate::billing_admin_provider_events::router())
        .merge(crate::billing_platform_center::router())
        .merge(crate::billing_runbooks::router())
        .merge(crate::command_center::router())
        .merge(crate::communications_center::router())
        .merge(crate::compliance_center::router())
        .merge(crate::customer_center::router())
        .merge(crate::developer_center::router())
        .merge(crate::entitlements_center::router())
        .merge(crate::global_search::router())
        .merge(crate::identity_governance_center::router())
        .merge(crate::operations_center::router())
        .merge(crate::revenue_center::router())
        .merge(crate::region_center::router())
        .merge(crate::risk_decision_center::router())
        .merge(crate::security_center::router())
        .merge(crate::tenants::router())
        .merge(crate::usage_center::router())
        .merge(crate::users::router())
        .merge(crate::workspaces::router())
        .layer(axum::middleware::from_fn_with_state(
            config.clone(),
            nvbes_core::http::internal_observability::internal_observability_guard,
        ));

    Router::new()
        .route("/health", get(health))
        .merge(private_routes)
}

async fn health() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            release_id: option_env!("VERGEN_GIT_SHA").unwrap_or("dev"),
            status: "ok",
        }),
    )
}

pub async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    res
}
