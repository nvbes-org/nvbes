use crate::app::AppState;
use crate::http::error::AppError;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::WebauthnStartRequest;

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/webauthn/start", post(challenge_webauthn_start))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/webauthn/start",
    tag = "auth",
    request_body = WebauthnStartRequest,
    responses(
        (status = 200, description = "WebAuthn login challenge", body = crate::domains::auth::types::WebauthnAuthStartResult),
        (status = 401, description = "Invalid authentication state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_webauthn_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<WebauthnStartRequest>,
) -> Result<Json<crate::domains::auth::types::WebauthnAuthStartResult>, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_login_webauthn_start",
        &meta.rate_limit_ip_key(),
        10,
        std::time::Duration::from_secs(60),
        &format!("state:{}", request.state_token),
        5,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let (auth_state, principal_id) =
        super::require_mfa_state(&state.redis, request.state_token).await?;

    let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
    let (challenge_id, options) = crate::domains::auth::webauthn::start_login_authentication(
        &state.db,
        &state.redis,
        &webauthn,
        auth_state.id,
        principal_id,
    )
    .await?;

    Ok(Json(crate::domains::auth::types::WebauthnAuthStartResult {
        challenge_id,
        options,
    }))
}
