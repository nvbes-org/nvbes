use crate::app::AppState;
use crate::domains::auth::{email_verification, email_verification_change, token_hash};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header::SET_COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
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
    #[serde(default = "default_timezone")]
    timezone: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeVerificationRequest {
    email: String,
    #[serde(default = "default_timezone")]
    timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
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
        nvbes_core::limiter::RateLimitRule {
            key: &format!(
                "ip:{}",
                crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
            ),
            max_hits: 10,
            window: std::time::Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &format!("key:{}", request.email),
            max_hits: 4,
            window: std::time::Duration::from_secs(300),
        },
    )
    .await?;

    let result = email_verification::resend_verification_email(
        &state.db,
        &state.redis,
        &state.config,
        &request.email,
        &request.timezone,
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
        (status = 401, description = "Registration enrollment missing, expired, or already used", body = ErrorEnvelope),
        (status = 409, description = "Email already exists", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn change_verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChangeVerificationRequest>,
) -> Result<Response, AppError> {
    let enrollment_token = crate::http::request::registration_enrollment_token(&headers)?;
    let rate_limit_key = format!("enrollment:{}", token_hash(&enrollment_token));

    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_verify_email_change",
        nvbes_core::limiter::RateLimitRule {
            key: &format!(
                "ip:{}",
                crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
            ),
            max_hits: 10,
            window: std::time::Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &rate_limit_key,
            max_hits: 4,
            window: std::time::Duration::from_secs(300),
        },
    )
    .await?;

    let result = email_verification_change::change_verification_email(
        &state.db,
        &state.redis,
        &state.config,
        &enrollment_token,
        &request.email,
        &request.timezone,
    )
    .await?;

    let secure_cookie = state.config.environment != "development";
    let mut response = Json(result).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        crate::http::cookies::clear_registration_enrollment_cookie(secure_cookie)?,
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header::COOKIE};

    #[test]
    fn current_email_without_enrollment_cookie_is_rejected() {
        let error = crate::http::request::registration_enrollment_token(&HeaderMap::new())
            .expect_err("an email address alone must not authorize enrollment changes");

        assert_eq!(error.code, "registration_enrollment_required");
        assert_eq!(error.status, axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn enrollment_cookie_is_accepted_as_the_credential() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("registration_enrollment=opaque-token"),
        );

        assert_eq!(
            crate::http::request::registration_enrollment_token(&headers)
                .expect("enrollment cookie should be accepted"),
            "opaque-token"
        );
    }
}
