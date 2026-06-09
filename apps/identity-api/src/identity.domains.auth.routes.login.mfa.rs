use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::auth::state::delete_state;
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::MfaRequest;

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/mfa", post(challenge_mfa))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/mfa",
    tag = "auth",
    request_body = MfaRequest,
    responses(
        (status = 200, description = "Login successful after MFA", body = crate::domains::auth::types::LoginResult),
        (status = 401, description = "Invalid credentials or state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_mfa(
    State(state): State<AppState>,
    Query(query): Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<MfaRequest>,
) -> Result<Response, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_login_mfa",
        nvbes_core::limiter::RateLimitRule {
            key: &meta.rate_limit_ip_key(),
            max_hits: 20,
            window: std::time::Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &format!("state:{}", request.state_token),
            max_hits: 8,
            window: std::time::Duration::from_secs(300),
        },
    )
    .await?;

    let (auth_state, principal_id) =
        super::require_mfa_state(&state.redis, request.state_token).await?;

    let authenticated_method = super::mfa_flow::resolve_authenticated_method(
        &state.db,
        &state.redis,
        &state.config,
        &request,
        auth_state.id,
        principal_id,
    )
    .await?;

    let result = sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.jwt,
        &state.config,
        principal_id,
        super::login_session_context(
            auth_state.email,
            &meta,
            auth_state.device_fingerprint,
            vec!["pwd".to_string(), authenticated_method],
            "aal2",
        ),
    )
    .await?;

    let secure_cookie = state.config.environment != "development";
    let authuser = query.authuser.as_deref().unwrap_or("0");
    let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
    let response = super::login_response(result, authuser, secure_cookie, session_expires_in)?;
    delete_state(&state.redis, request.state_token).await?;
    Ok(response)
}
