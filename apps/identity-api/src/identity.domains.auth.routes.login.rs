use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::auth::state::{AuthState, create_state, fetch_state};
use crate::domains::auth::types::LoginResult;
use crate::http::cookies::{
    auth_cookie, auth_cookie_name_with_user, csrf_cookie, generate_csrf_token,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    http::HeaderMap,
    http::{StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use serde_json::Value;
use uuid::Uuid;

#[path = "identity.domains.auth.routes.login.identifier.rs"]
pub mod identifier;
#[path = "identity.domains.auth.routes.login.identifier_flow.rs"]
pub mod identifier_flow;
#[path = "identity.domains.auth.routes.login.identifier_guard.rs"]
pub mod identifier_guard;
#[path = "identity.domains.auth.routes.login.mfa.rs"]
pub mod mfa;
#[path = "identity.domains.auth.routes.login.mfa_flow.rs"]
pub mod mfa_flow;
#[path = "identity.domains.auth.routes.login.pwd.rs"]
pub mod pwd;
#[path = "identity.domains.auth.routes.login.webauthn.rs"]
pub mod webauthn;

#[path = "identity.domains.auth.routes.login.types.rs"]
mod types;

#[derive(serde::Deserialize)]
pub(crate) struct LoginQuery {
    pub(crate) authuser: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(identifier::router())
        .merge(pwd::router())
        .merge(webauthn::router())
        .merge(mfa::router())
}

pub(crate) struct LoginRequestMeta {
    ip: Option<String>,
    user_agent: Option<String>,
}

impl LoginRequestMeta {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            ip: crate::http::request::client_ip(headers),
            user_agent: crate::http::request::user_agent(headers),
        }
    }

    pub(crate) fn ip(&self) -> Option<String> {
        self.ip.clone()
    }

    pub(crate) fn user_agent(&self) -> Option<String> {
        self.user_agent.clone()
    }

    pub(crate) fn rate_limit_ip_key(&self) -> String {
        format!(
            "ip:{}",
            self.ip.clone().unwrap_or_else(|| "unknown".to_string())
        )
    }
}

pub(crate) fn login_session_context(
    email: String,
    meta: &LoginRequestMeta,
    device_fingerprint: Option<Value>,
    amr: Vec<String>,
    acr: &'static str,
) -> sessions::LoginSessionContext {
    sessions::LoginSessionContext {
        email,
        ip: meta.ip(),
        user_agent: meta.user_agent(),
        device_fingerprint,
        amr,
        acr,
    }
}

pub(crate) fn login_response(
    result: LoginResult,
    authuser: &str,
    secure_cookie: bool,
    session_expires_in: i64,
) -> Result<Response, AppError> {
    let session_cookie_name = auth_cookie_name_with_user("session", authuser, secure_cookie);
    let session_cookie_value = result.session_token.clone();
    let csrf_token = generate_csrf_token();
    let csrf_cookie_name = auth_cookie_name_with_user("csrf_token", authuser, secure_cookie);

    let mut response = (StatusCode::OK, Json(result)).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        auth_cookie(
            &session_cookie_name,
            &session_cookie_value,
            session_expires_in,
            secure_cookie,
        )?,
    );
    response.headers_mut().append(
        SET_COOKIE,
        csrf_cookie(
            &csrf_cookie_name,
            &csrf_token,
            session_expires_in,
            secure_cookie,
        )?,
    );

    Ok(response)
}

pub(crate) async fn challenge_response(
    redis: &nvbes_redis::RedisPool,
    status: StatusCode,
    principal_id: Option<Uuid>,
    email: &str,
    next_step: &str,
    device_fingerprint: Option<Value>,
    available_methods: Option<Vec<String>>,
) -> Result<Response, AppError> {
    let state_token = create_state(
        redis,
        principal_id,
        email,
        next_step,
        device_fingerprint,
        None,
    )
    .await?;

    Ok((
        status,
        Json(types::IdentifierResult {
            next_step: next_step.to_string(),
            state_token,
            available_methods,
        }),
    )
        .into_response())
}

pub(crate) async fn require_mfa_state(
    redis: &nvbes_redis::RedisPool,
    state_token: Uuid,
) -> Result<(AuthState, Uuid), AppError> {
    let auth_state = fetch_state(redis, state_token, "mfa").await?;
    let principal_id = auth_state.principal_id.ok_or_else(|| {
        AppError::unauthorized(
            "invalid_auth_state",
            "The authentication session is invalid or has expired.",
        )
    })?;
    Ok((auth_state, principal_id))
}

pub(crate) fn validation_failed(message: &'static str) -> AppError {
    AppError::bad_request("validation_failed", message)
}

pub(crate) fn challenge_locked() -> AppError {
    AppError::forbidden(
        "challenge_locked",
        "The WebAuthn challenge has been locked after repeated failures.",
    )
}
