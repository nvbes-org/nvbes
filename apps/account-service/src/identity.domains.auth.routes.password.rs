use crate::app::AppState;
use crate::domains::auth::exposed_credentials;
use crate::domains::auth::password;
use crate::domains::auth::types::ChangePasswordInput;
use crate::http::error::AppError;
use crate::http::middleware::jwt::{
    AuthContext,
    account_access::{self, AccountAccess, SECURITY_WRITE_SCOPE},
};
use crate::http::request::{client_ip, user_agent};
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use utoipa::ToSchema;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/password/forgot", post(forgot_password))
        .route("/password/reset", post(reset_password))
        .route(
            "/password/change",
            account_access::protected_method(
                _state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(change_password),
            ),
        )
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ForgotPasswordRequest {
    email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

#[utoipa::path(
    post,
    path = "/auth/password/change",
    tag = "auth",
    request_body = ChangePasswordInput,
    responses(
        (status = 200, description = "Password changed successfully", body = crate::domains::auth::types::ChangePasswordResult),
        (status = 400, description = "Validation error or same password", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<ChangePasswordInput>,
) -> Result<Json<crate::domains::auth::types::ChangePasswordResult>, AppError> {
    exposed_credentials::check_new_password(&headers)?;

    crate::domains::auth::verification::require_password_change_step_up(&state.redis, &auth)
        .await?;

    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_change_password",
        &format!("user:{}", auth.user_id),
        5,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let result = password::change(
        &state.db,
        &state.redis,
        &state.config,
        auth.user_id,
        auth.session_id,
        request,
    )
    .await?;
    crate::domains::auth::verification::clear_password_change_step_up(&state.redis, &auth).await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/auth/password/forgot",
    tag = "auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset initiated", body = crate::domains::auth::types::ForgotPasswordResult),
        (status = 400, description = "Validation error", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Json<crate::domains::auth::types::ForgotPasswordResult>, AppError> {
    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "auth_forgot_password",
        &request.email,
        15,
        5,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let result = password::forgot(
        &state.db,
        &state.redis,
        &state.config,
        crate::domains::auth::types::ForgotPasswordInput {
            email: request.email,
        },
        state.config.auth_password_reset_ttl_minutes,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/auth/password/reset",
    tag = "auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful", body = crate::domains::auth::types::ResetPasswordResult),
        (status = 400, description = "Invalid or expired token", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Response, AppError> {
    exposed_credentials::check_new_password(&headers)?;

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "auth_reset_password",
        &request.token,
        20,
        8,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let result = password::reset(
        &state.db,
        &state.redis,
        &state.config,
        crate::domains::auth::types::ResetPasswordInput {
            token: request.token,
            new_password: request.new_password,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    reset_password_response(result, state.config.environment != "development")
}

fn reset_password_response(
    result: crate::domains::auth::types::ResetPasswordResult,
    secure_cookie: bool,
) -> Result<Response, AppError> {
    let mut response = (StatusCode::OK, Json(result)).into_response();
    for header in [
        crate::http::cookies::auth_cookie(
            &crate::http::cookies::auth_cookie_name("session", secure_cookie),
            "",
            0,
            secure_cookie,
        )?,
        crate::http::cookies::csrf_cookie(
            &crate::http::cookies::auth_cookie_name("csrf_token", secure_cookie),
            "",
            0,
            secure_cookie,
        )?,
        crate::http::cookies::auth_cookie(
            &crate::http::cookies::auth_cookie_name("device", secure_cookie),
            "",
            0,
            secure_cookie,
        )?,
    ] {
        response.headers_mut().append(SET_COOKIE, header);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::reset_password_response;
    use axum::http::header::SET_COOKIE;

    #[test]
    fn password_reset_response_expires_revoked_browser_credentials() {
        let response = reset_password_response(
            crate::domains::auth::types::ResetPasswordResult { success: true },
            true,
        )
        .expect("password reset response should be valid");
        let cookies = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|value| value.to_str().expect("cookie should be text"))
            .collect::<Vec<_>>();

        assert_eq!(cookies.len(), 3);
        for name in ["__Host-session", "__Host-csrf_token", "__Host-device"] {
            let cookie = cookies
                .iter()
                .find(|cookie| cookie.starts_with(&format!("{name}=;")))
                .unwrap_or_else(|| panic!("missing expiration cookie for {name}"));
            assert!(cookie.contains("Max-Age=0"));
            assert!(cookie.contains("SameSite=Strict"));
            assert!(cookie.contains("Path=/"));
            assert!(cookie.contains("Secure"));
            assert!(!cookie.contains("Domain="));
        }
    }
}
