use crate::app::AppState;
use crate::http::error::AppError;
use axum::{Json, Router, extract::State, http::HeaderMap, response::Response, routing::post};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::{WebauthnDiscoverableFinishRequest, WebauthnStartRequest};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/challenge/webauthn/start", post(challenge_webauthn_start))
        .route(
            "/challenge/webauthn/discoverable/start",
            post(challenge_webauthn_discoverable_start),
        )
        .route(
            "/challenge/webauthn/discoverable/finish",
            post(challenge_webauthn_discoverable_finish),
        )
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
        nvbes_core::limiter::RateLimitRule {
            key: &meta.rate_limit_ip_key(),
            max_hits: 10,
            window: std::time::Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &format!("state:{}", request.state_token),
            max_hits: 5,
            window: std::time::Duration::from_secs(300),
        },
    )
    .await?;

    let (auth_state, principal_id) =
        super::require_mfa_state(&state.redis, request.state_token).await?;

    let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
    let ip = meta.ip();
    let user_agent = meta.user_agent();
    let (challenge_id, options) = crate::domains::auth::webauthn::start_login_authentication(
        &state.db,
        &state.redis,
        &webauthn,
        auth_state.id,
        principal_id,
        ip.as_deref(),
        user_agent.as_deref(),
    )
    .await?;

    Ok(Json(crate::domains::auth::types::WebauthnAuthStartResult {
        challenge_id,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/webauthn/discoverable/start",
    tag = "auth",
    responses(
        (status = 200, description = "Discoverable WebAuthn login challenge", body = crate::domains::auth::types::WebauthnAuthStartResult),
        (status = 401, description = "Invalid authentication state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_webauthn_discoverable_start(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::domains::auth::types::WebauthnAuthStartResult>, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login_webauthn_discoverable_start",
        &meta.rate_limit_ip_key(),
        20,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
    let (challenge_id, options) =
        crate::domains::auth::webauthn::start_discoverable_login_authentication(
            &state.redis,
            &webauthn,
        )
        .await?;

    Ok(Json(crate::domains::auth::types::WebauthnAuthStartResult {
        challenge_id,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/webauthn/discoverable/finish",
    tag = "auth",
    request_body = WebauthnDiscoverableFinishRequest,
    responses(
        (status = 200, description = "Login successful after discoverable WebAuthn", body = crate::domains::auth::types::LoginResult),
        (status = 401, description = "Invalid authentication state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_webauthn_discoverable_finish(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<WebauthnDiscoverableFinishRequest>,
) -> Result<Response, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_login_webauthn_discoverable_finish",
        nvbes_core::limiter::RateLimitRule {
            key: &meta.rate_limit_ip_key(),
            max_hits: 20,
            window: std::time::Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &format!("challenge:{}", request.challenge_id),
            max_hits: 8,
            window: std::time::Duration::from_secs(300),
        },
    )
    .await?;

    let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
    let ip = meta.ip();
    let user_agent = meta.user_agent();
    let (principal_id, email, method) =
        crate::domains::auth::webauthn::finish_discoverable_login_authentication(
            &state.db,
            &state.redis,
            &webauthn,
            request.challenge_id,
            &request.webauthn_response,
            ip.as_deref(),
            user_agent.as_deref(),
        )
        .await?;

    let result = crate::domains::auth::sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.jwt,
        &state.config,
        principal_id,
        super::login_session_context(email, &meta, None, vec![method], "aal2"),
    )
    .await?;

    let secure_cookie = state.config.environment != "development";
    let authuser = query.authuser.as_deref().unwrap_or("0");
    let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
    super::login_response(result, authuser, secure_cookie, session_expires_in)
}
