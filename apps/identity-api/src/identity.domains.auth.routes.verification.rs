use crate::app::AppState;
use crate::domains::auth::{email_verification, email_verification_change, sessions};
use crate::http::error::AppError;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use utoipa::ToSchema;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify-email/resend", post(resend_verify_email))
        .route("/verify-email/change", post(change_verify_email))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ResendVerificationRequest {
    email: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeVerificationRequest {
    current_email: Option<String>,
    email: String,
}

#[utoipa::path(
    post,
    path = "/auth/verify-email/resend",
    tag = "auth",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Verification email resent or cooldown reported", body = crate::domains::auth::types::ResendVerificationResult),
        (status = 400, description = "Validation error", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn resend_verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ResendVerificationRequest>,
) -> Result<Json<crate::domains::auth::types::ResendVerificationResult>, AppError> {
    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_verify_email_resend",
        &format!(
            "ip:{}",
            crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
        ),
        10,
        std::time::Duration::from_secs(60),
        &format!("key:{}", request.email),
        4,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let result = email_verification::resend_verification_email(
        &state.db,
        &state.redis,
        &state.config,
        &request.email,
    )
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/auth/verify-email/change",
    tag = "auth",
    request_body = ChangeVerificationRequest,
    responses(
        (status = 200, description = "Verification email changed and resent", body = crate::domains::auth::types::ResendVerificationResult),
        (status = 400, description = "Validation error", body = ErrorEnvelope),
        (status = 404, description = "Account not found", body = ErrorEnvelope),
        (status = 409, description = "Email already exists", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn change_verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChangeVerificationRequest>,
) -> Result<Json<crate::domains::auth::types::ResendVerificationResult>, AppError> {
    let auth =
        sessions::try_authenticate_bearer(&state.db, &state.redis, &state.jwt, &headers).await;

    let rate_limit_key = auth
        .as_ref()
        .map(|auth| format!("principal:{}", auth.user_id))
        .or_else(|| {
            request
                .current_email
                .as_ref()
                .map(|email| format!("key:{email}"))
        })
        .unwrap_or_else(|| format!("key:{}", request.email));

    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_verify_email_change",
        &format!(
            "ip:{}",
            crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
        ),
        10,
        std::time::Duration::from_secs(60),
        &rate_limit_key,
        4,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let result = email_verification_change::change_verification_email(
        &state.db,
        &state.redis,
        &state.config,
        auth.as_ref().map(|auth| auth.user_id),
        request.current_email.as_deref(),
        &request.email,
    )
    .await?;

    Ok(Json(result))
}
