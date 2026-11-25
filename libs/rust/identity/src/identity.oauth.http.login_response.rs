use super::{OAuthError, ProtocolError};
use crate::browser::{BrowserProof, BrowserSecurity};
use axum::{
    Json,
    response::{IntoResponse, Response},
};

/// Only call after the authentication and OAuth-binding transaction commits.
pub(super) fn authenticated_response(
    browser: &BrowserSecurity,
    proof: &BrowserProof,
    interaction: &str,
    session: &str,
    csrf: &str,
) -> Result<Response, ProtocolError> {
    let cookie = browser
        .session_cookie(session)
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let session_csrf = browser
        .session_csrf_token(session, &proof.browser_token)
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    Ok(([("set-cookie",cookie),("cache-control","no-store".parse().unwrap())],
        Json(serde_json::json!({"interaction":interaction,"csrf_token":csrf,"session_csrf_token":session_csrf}))).into_response())
}
