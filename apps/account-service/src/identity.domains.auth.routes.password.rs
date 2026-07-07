use crate::app::AppState;
use crate::domains::auth::exposed_credentials;
use crate::domains::auth::password;
use crate::domains::auth::types::ChangePasswordInput;
use crate::domains::auth::verification::require_recent_step_up;
use crate::http::error::AppError;
use crate::http::middleware::jwt::{AuthContext, jwt_auth_middleware};
use crate::http::request::{client_ip, user_agent};
use axum::{Json, Router, extract::Extension, extract::State, http::HeaderMap, routing::post};
use nvbes_core::auth::Aal;
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use utoipa::ToSchema;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/password/forgot", post(forgot_password))
        .route("/password/reset", post(reset_password))
        .route(
            "/password/change",
            post(change_password).layer(axum::middleware::from_fn_with_state(
                _state.clone(),
                jwt_auth_middleware,
            )),
        )
        .route(
            "/password/recovery/approve",
            post(approve_recovery).layer(axum::middleware::from_fn_with_state(
                _state.clone(),
                jwt_auth_middleware,
            )),
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

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ApproveRecoveryRequest {
    email: String,
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
        &state.config.environment,
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
) -> Result<Json<crate::domains::auth::types::ResetPasswordResult>, AppError> {
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

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/auth/password/recovery/approve",
    tag = "auth",
    request_body = ApproveRecoveryRequest,
    responses(
        (status = 200, description = "Recovery approved", body = crate::domains::auth::types::ApproveRecoveryResult),
        (status = 401, description = "Unauthorized or insufficient AAL", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn approve_recovery(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(auth): Extension<crate::http::middleware::jwt::AuthContext>,
    Json(request): Json<ApproveRecoveryRequest>,
) -> Result<Json<crate::domains::auth::types::ApproveRecoveryResult>, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_recovery_approve",
        &format!(
            "ip:{}",
            crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
        ),
        10,
        std::time::Duration::from_secs(300),
    )
    .await?;

    require_recent_step_up(&state.redis, &auth, Some(approve_recovery_required_aal())).await?;

    let result = password::approve_enterprise_recovery(
        &state.db,
        &state.redis,
        auth.user_id,
        &request.email,
        state.config.auth_password_reset_ttl_minutes,
        &state.config.environment,
    )
    .await?;

    Ok(Json(result))
}

fn approve_recovery_required_aal() -> Aal {
    Aal::Aal2
}

#[cfg(test)]
mod tests {
    use super::approve_recovery_required_aal;
    use nvbes_core::auth::Aal;

    #[test]
    fn approve_recovery_requires_aal2_step_up() {
        assert_eq!(approve_recovery_required_aal(), Aal::Aal2);
    }
}
