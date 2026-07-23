use crate::app::AppState;
use crate::domains::legal::db::UserConsent;
use crate::domains::legal::service;
use crate::http::error::AppError;
use crate::http::middleware::jwt::{AuthContext, jwt_auth_middleware};
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::HeaderMap,
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use utoipa::ToSchema;

pub fn router(state: &AppState) -> Router<AppState> {
    let auth_middleware = jwt_auth_middleware;

    Router::new()
        .route(
            "/legal/consent",
            post(grant_consent).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/legal/consent/revoke",
            post(revoke_consent).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/legal/consents",
            get(list_consents).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/legal/gpc",
            get(gpc_status).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
}

#[derive(Deserialize, ToSchema)]
pub struct ConsentRequest {
    pub consent_type: String,
    pub document_version: String,
}

#[utoipa::path(
    post,
    path = "/legal/consent",
    tag = "legal",
    request_body = ConsentRequest,
    responses(
        (status = 200, description = "Consent successfully recorded", body = UserConsent),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn grant_consent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(payload): Json<ConsentRequest>,
) -> Result<Json<UserConsent>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "missing_tenant",
            "Tenant ID is required for legal consent tracking",
        )
    })?;

    let ip_address = crate::http::request::client_ip(&headers);
    let user_agent = crate::http::request::user_agent(&headers);

    let consent = service::grant_consent(
        &state.db,
        auth.user_id,
        tenant_id,
        auth.workspace_id,
        &payload.consent_type,
        &payload.document_version,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await?;

    Ok(Json(consent))
}

#[utoipa::path(
    post,
    path = "/legal/consent/revoke",
    tag = "legal",
    request_body = ConsentRequest,
    responses(
        (status = 200, description = "Consent successfully revoked"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_consent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(payload): Json<ConsentRequest>,
) -> Result<StatusCodeResponse, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "missing_tenant",
            "Tenant ID is required for legal consent revocation",
        )
    })?;

    let ip_address = crate::http::request::client_ip(&headers);
    let user_agent = crate::http::request::user_agent(&headers);

    service::revoke_consent(
        &state.db,
        auth.user_id,
        tenant_id,
        auth.workspace_id,
        &payload.consent_type,
        &payload.document_version,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await?;

    Ok(StatusCodeResponse)
}

#[utoipa::path(
    get,
    path = "/legal/consents",
    tag = "legal",
    responses(
        (status = 200, description = "Consent history listed successfully", body = service::ConsentHistoryResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Max results"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
    ),
)]
pub(crate) async fn list_consents(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Query(query): axum::extract::Query<ListConsentsQuery>,
) -> Result<Json<service::ConsentHistoryResult>, AppError> {
    let result = service::list_consents(&state.db, auth.user_id, query.limit, query.cursor).await?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub(crate) struct ListConsentsQuery {
    limit: Option<i64>,
    cursor: Option<String>,
}

// Minimal helper to return 200 OK without any body content
pub(crate) struct StatusCodeResponse;

impl axum::response::IntoResponse for StatusCodeResponse {
    fn into_response(self) -> axum::response::Response {
        axum::http::StatusCode::OK.into_response()
    }
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpcStatusResponse {
    pub gpc_enabled: bool,
    pub gpc_opt_out_active: bool,
}

#[utoipa::path(
    get,
    path = "/legal/gpc",
    tag = "legal",
    responses(
        (status = 200, description = "GPC status and auto-recording result", body = GpcStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn gpc_status(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
) -> Result<Json<GpcStatusResponse>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "missing_tenant",
            "Tenant ID is required for GPC consent tracking",
        )
    })?;

    let gpc_detected = crate::http::request::gpc_enabled(&headers);

    if gpc_detected {
        let ip_address = crate::http::request::client_ip(&headers);
        let user_agent = crate::http::request::user_agent(&headers);

        service::record_gpc_opt_out(
            &state.db,
            auth.user_id,
            tenant_id,
            auth.workspace_id,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await?;
    }

    let gpc_opt_out_active =
        super::db::is_consent_active(&state.db, auth.user_id, "gpc_opt_out", "gpc_v1").await?;

    Ok(Json(GpcStatusResponse {
        gpc_enabled: gpc_detected,
        gpc_opt_out_active,
    }))
}
