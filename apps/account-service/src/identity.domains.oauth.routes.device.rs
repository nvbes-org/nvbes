use axum::{Extension, Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;

use crate::{
    app::AppState,
    http::{
        error::AppError,
        middleware::jwt::{
            AuthContext,
            account_access::{self, AccountAccess, OAUTH_APPROVAL_SCOPE},
        },
    },
};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/authorize", post(device_authorize))
        .route("/verify", post(device_verify))
        .route(
            "/approve",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_APPROVAL_SCOPE),
                post(device_approve),
            ),
        )
        .route(
            "/deny",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_APPROVAL_SCOPE),
                post(device_deny),
            ),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeviceAuthorizeRequest {
    client_id: String,
    scope: String,
    audience: Option<String>,
    resource: Option<Vec<String>>,
}

#[utoipa::path(
    post,
    path = "/oauth/device/authorize",
    tag = "oauth",
    request_body = DeviceAuthorizeRequest,
    responses(
        (status = 200, description = "Device authorization response", body = crate::domains::oauth::service::DeviceAuthorizationView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn device_authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeviceAuthorizeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = crate::domains::oauth::device_authorization::device_authorization(
        &state.db,
        &state.redis,
        &state.jwt,
        &state.config.web_base_url,
        &headers,
        crate::domains::oauth::service::DeviceAuthorizationInput {
            client_id: request.client_id,
            scope: request.scope,
            audience: request.audience,
            resource_indicators: request.resource.unwrap_or_default(),
        },
    )
    .await?;

    Ok(Json(serde_json::to_value(result)?))
}

#[utoipa::path(
    post,
    path = "/oauth/device/verify",
    tag = "oauth",
    request_body = crate::domains::oauth::service::DeviceVerificationInput,
    responses(
        (status = 200, description = "Device verification response", body = crate::domains::oauth::service::DeviceVerificationView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn device_verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<crate::domains::oauth::service::DeviceVerificationInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = crate::domains::oauth::device_verification::verify_device_code(
        &state.db,
        &state.redis,
        &headers,
        request,
    )
    .await?;
    Ok(Json(serde_json::to_value(result)?))
}

#[utoipa::path(
    post,
    path = "/oauth/device/approve",
    tag = "oauth",
    request_body = crate::domains::oauth::service::DeviceApprovalInput,
    responses(
        (status = 200, description = "Device code approved"),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn device_approve(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::oauth::service::DeviceApprovalInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    crate::domains::oauth::device_verification::approve_device_code(
        &state.db,
        &state.redis,
        &auth,
        request,
    )
    .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[utoipa::path(
    post,
    path = "/oauth/device/deny",
    tag = "oauth",
    request_body = crate::domains::oauth::service::DeviceVerificationInput,
    responses(
        (status = 200, description = "Device code denied"),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn device_deny(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::oauth::service::DeviceVerificationInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    crate::domains::oauth::device_verification::deny_device_code(&state.redis, &auth, request)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
