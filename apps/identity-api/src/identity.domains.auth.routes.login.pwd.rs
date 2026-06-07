use crate::app::AppState;
use crate::domains::auth::{
    sessions,
    state::{delete_state, fetch_state},
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::{IdentifierResult, PwdRequest};

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/pwd", post(challenge_pwd))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/pwd",
    tag = "auth",
    request_body = PwdRequest,
    responses(
        (status = 200, description = "Login successful", body = crate::domains::auth::types::LoginResult),
        (status = 202, description = "Password accepted, MFA challenge required", body = IdentifierResult),
        (status = 401, description = "Invalid credentials or state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_pwd(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<PwdRequest>,
) -> Result<Response, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    let auth_state = fetch_state(&state.redis, request.state_token, "pwd").await?;
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login",
        &format!("key:{}", auth_state.email),
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let login_input = crate::domains::auth::types::LoginInput {
        email: auth_state.email.clone(),
        password: request.password,
        ip: meta.ip(),
        user_agent: meta.user_agent(),
        device_fingerprint: auth_state.device_fingerprint.clone(),
    };
    let verified =
        sessions::verify_primary_credentials(&state.db, &state.redis, &state.config, &login_input)
            .await?;
    if let Some(available_methods) =
        super::identifier_flow::resolve_post_password_challenge(&state.db, verified.principal_id)
            .await?
    {
        let response = super::challenge_response(
            &state.redis,
            StatusCode::ACCEPTED,
            Some(verified.principal_id),
            &auth_state.email,
            "mfa",
            auth_state.device_fingerprint,
            Some(available_methods),
        )
        .await?;
        delete_state(&state.redis, request.state_token).await?;
        return Ok(response);
    }

    let result = sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.jwt,
        &state.config,
        verified.principal_id,
        super::login_session_context(
            verified.email,
            &meta,
            auth_state.device_fingerprint,
            vec!["pwd".to_string()],
            "aal1",
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
