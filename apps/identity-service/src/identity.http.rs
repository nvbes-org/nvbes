use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::IdentityState;
use crate::auth::{authenticate, register_password_identity, revoke_session_token};
use crate::session::{
    append_cleared_session_cookie, append_session_cookie, session_token_from_headers,
    validate_return_to,
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub principal_id: Uuid,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub return_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub session_token: String,
    pub principal_id: Uuid,
    pub expires_in_hours: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_url: Option<String>,
}

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .with_state(state.clone())
}

async fn register(
    State(state): State<IdentityState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), HttpError> {
    if !state.config.public_signup_enabled {
        return Err(HttpError::Forbidden(
            "Public signup is closed until an explicit GO".to_string(),
        ));
    }

    let principal_id =
        register_password_identity(&state.db, &req.email, &req.password, "identity.registered")
            .await
            .map_err(map_register_error)?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            principal_id,
            message: "Registration successful".to_string(),
        }),
    ))
}

async fn login(
    State(state): State<IdentityState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let session = authenticate(&state.db, &req.email, &req.password)
        .await
        .map_err(|_| HttpError::Unauthorized("authentication failed".to_string()))?;

    let continue_url = req
        .return_to
        .as_deref()
        .and_then(|value| validate_return_to(value, &state.config.token_issuer));

    let body = LoginResponse {
        session_token: session.session_token.clone(),
        principal_id: session.principal_id,
        expires_in_hours: crate::session::SESSION_TTL_HOURS,
        continue_url,
    };

    let mut headers = HeaderMap::new();
    append_session_cookie(
        &mut headers,
        &session.session_token,
        state.config.session_cookie_secure,
    );

    Ok((StatusCode::OK, headers, Json(body)))
}

async fn logout(State(state): State<IdentityState>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(token) = session_token_from_headers(&headers) {
        let _ = revoke_session_token(&state.db, &token).await;
    }

    let mut response_headers = HeaderMap::new();
    append_cleared_session_cookie(&mut response_headers, state.config.session_cookie_secure);
    (
        StatusCode::NO_CONTENT,
        response_headers,
        // Explicit empty body keeps axum from inventing content.
        (),
    )
}

fn map_register_error(error: anyhow::Error) -> HttpError {
    let message = error.to_string();
    if message.contains("password") || message.contains("email") {
        HttpError::BadRequest(message)
    } else if message.contains("duplicate") || message.contains("unique") {
        HttpError::Conflict("Email is already registered".to_string())
    } else {
        HttpError::InternalServerError(message)
    }
}

#[derive(Debug)]
enum HttpError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    InternalServerError(String),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            HttpError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            HttpError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            HttpError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            HttpError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            HttpError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
#[path = "identity.http.tests.rs"]
mod tests;
