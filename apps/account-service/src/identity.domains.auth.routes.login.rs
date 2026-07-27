use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::auth::state::{AuthState, create_state, fetch_state};
use crate::domains::auth::types::LoginResult;
use crate::http::cookies::{
    auth_cookie, auth_cookie_name, auth_cookie_name_with_user, csrf_cookie, generate_csrf_token,
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

impl LoginQuery {
    pub(crate) fn authuser(&self) -> Result<&str, AppError> {
        crate::http::authuser::normalize(self.authuser.as_deref())
    }
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
    request_profile: sessions::cookie_theft::SessionRequestProfile,
    installation_token: Option<String>,
}

impl LoginRequestMeta {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        let request_profile = sessions::cookie_theft::SessionRequestProfile::from_headers(headers);
        Self {
            ip: request_profile.ip.clone(),
            user_agent: request_profile.user_agent.clone(),
            request_profile,
            installation_token: crate::http::request::cookie_value(
                headers,
                &["__Host-device=", "device="],
            ),
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

    pub(crate) fn installation_token(&self) -> Option<&str> {
        self.installation_token.as_deref()
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
        request_profile: meta.request_profile.clone(),
        installation_token: meta.installation_token.clone(),
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
    csrf_secret: &str,
    product_analytics: &nvbes_product_analytics::ProductAnalytics,
    request_headers: &HeaderMap,
) -> Result<Response, AppError> {
    let (distinct_id, session_id) =
        crate::http::request::product_analytics_correlation(request_headers);
    product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::user(
            "auth.login_completed",
            result.user.id,
        )
        .correlation(distinct_id, session_id),
    );

    let session_cookie_name = auth_cookie_name_with_user("session", authuser, secure_cookie);
    let session_cookie_value = result.browser_session_token.clone();
    let device_cookie_token = result.device_cookie_token.clone();
    let csrf_token = generate_csrf_token(&session_cookie_value, csrf_secret);
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
    if let Some(device_cookie_token) = device_cookie_token {
        response.headers_mut().append(
            SET_COOKIE,
            auth_cookie(
                &auth_cookie_name("device", secure_cookie),
                &device_cookie_token,
                60 * 60 * 24 * 180,
                secure_cookie,
            )?,
        );
    }
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
    let state_token =
        create_state(redis, principal_id, email, next_step, device_fingerprint).await?;

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
