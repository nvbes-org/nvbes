use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Serialize;

use super::csp_report;
use super::middleware::{csrf, dpop, idempotency, origin};
use super::observability;
use super::openapi;
use crate::{app::AppState, http::error::AppError};

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
        .route("/observability/sentry-smoke", post(sentry_smoke))
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
            get(change_password_well_known),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(crate::domains::oauth::metadata::oauth_authorization_server_metadata),
        )
        .route(
            "/.well-known/openid-configuration",
            get(crate::domains::oauth::metadata::openid_configuration),
        )
        .route("/.well-known/gpc.json", get(gpc_well_known))
        .route(
            "/.well-known/passkey-endpoints",
            get(passkey_endpoints_well_known),
        )
        .route("/.well-known/webauthn", get(webauthn_well_known))
        .route("/.well-known/security.txt", get(security_txt_well_known))
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

async fn sentry_smoke(
    State(state): State<AppState>,
) -> Json<nvbes_observability::SentrySmokeResult> {
    Json(nvbes_observability::capture_sentry_smoke(
        &state.config.app_name,
        &state.config.environment,
        "api",
        state.config.sentry_dsn.is_some(),
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

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
struct ChangePasswordWellKnownResponse {
    change_password: String,
}

#[utoipa::path(
    get,
    path = "/.well-known/change-password",
    tag = "auth",
    responses(
        (status = 200, description = "Password change URL", body = ChangePasswordWellKnownResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn change_password_well_known(
    State(state): State<AppState>,
) -> Result<Json<ChangePasswordWellKnownResponse>, AppError> {
    Ok(Json(ChangePasswordWellKnownResponse {
        change_password: change_password_url(&state.config.web_base_url),
    }))
}

fn change_password_url(web_base_url: &str) -> String {
    format!(
        "{}/account/security/password",
        web_base_url.trim_end_matches('/')
    )
}

#[derive(Serialize, utoipa::ToSchema)]
struct GpcWellKnownResponse {
    gpc: bool,
    version: u32,
}

#[utoipa::path(
    get,
    path = "/.well-known/gpc.json",
    tag = "auth",
    responses(
        (status = 200, description = "GPC support status", body = GpcWellKnownResponse),
    ),
)]
async fn gpc_well_known() -> Json<GpcWellKnownResponse> {
    Json(GpcWellKnownResponse {
        gpc: true,
        version: 1,
    })
}

#[derive(Serialize, utoipa::ToSchema)]
struct PasskeyEndpointsWellKnownResponse {
    enroll: String,
    manage: String,
}

#[utoipa::path(
    get,
    path = "/.well-known/passkey-endpoints",
    tag = "auth",
    responses(
        (status = 200, description = "Passkey enrollment and management endpoints", body = PasskeyEndpointsWellKnownResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn passkey_endpoints_well_known(
    State(state): State<AppState>,
) -> Result<Json<PasskeyEndpointsWellKnownResponse>, AppError> {
    let base = state.config.web_base_url.trim_end_matches('/').to_string();
    Ok(Json(PasskeyEndpointsWellKnownResponse {
        enroll: format!("{}/account/mfa/passkey/setup", base),
        manage: format!("{}/account/mfa", base),
    }))
}

#[derive(Serialize, utoipa::ToSchema)]
struct WebAuthnWellKnownResponse {
    origins: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/.well-known/webauthn",
    tag = "auth",
    responses(
        (status = 200, description = "WebAuthn related origins for cross-domain passkey sharing", body = WebAuthnWellKnownResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn webauthn_well_known(
    State(state): State<AppState>,
) -> Result<Json<WebAuthnWellKnownResponse>, AppError> {
    Ok(Json(WebAuthnWellKnownResponse {
        origins: state.config.webauthn_related_origins.clone(),
    }))
}

async fn security_txt_well_known(
    State(state): State<AppState>,
) -> Result<
    (
        axum::http::StatusCode,
        [(axum::http::HeaderName, &'static str); 1],
        String,
    ),
    AppError,
> {
    let base = state.config.web_base_url.trim_end_matches('/').to_string();
    let contact = match &state.config.security_contact_email {
        Some(email) => format!("mailto:{}", email),
        None => {
            let domain = state
                .config
                .web_base_url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap_or("nvbes.fr");
            format!("mailto:security@{}", domain)
        }
    };
    let policy = format!("{}/.well-known/security-policy", base);
    let expires = "2027-06-01T00:00:00.000Z";
    let body = format!(
        "Contact: {}\nPolicy: {}\nExpires: {}\n",
        contact, policy, expires
    );
    Ok((
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::WebAuthnWellKnownResponse;
    use super::change_password_url;

    #[test]
    fn webauthn_well_known_serializes_origins() {
        let response = WebAuthnWellKnownResponse {
            origins: vec!["https://drive.nvbes.io".into()],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"origins": ["https://drive.nvbes.io"]})
        );
    }

    #[test]
    fn change_password_url_trims_web_base_url() {
        assert_eq!(
            change_password_url("https://identity.example/"),
            "https://identity.example/account/security/password"
        );
    }
}
