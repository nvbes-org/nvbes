use crate::app::AppState;
use crate::domains::auth::state::fetch_state;
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
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login_webauthn_start",
        &format!(
            "ip:{}",
            crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
        ),
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login_webauthn_start",
        &format!("state:{}", request.state_token),
        5,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let auth_state = fetch_state(&state.redis, request.state_token, "mfa").await?;
    let principal_id = auth_state.principal_id.ok_or_else(|| {
        AppError::unauthorized(
            "invalid_auth_state",
            "The authentication session is invalid or has expired.",
        )
    })?;

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
