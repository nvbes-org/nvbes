use super::{AuthorizationState, OAuthError, ProtocolError, StepUpForm};
use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};

pub(super) async fn logout(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ProtocolError> {
    state
        .browser
        .verify_mutation(&axum::http::Method::POST, &headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    if let Some(session) = state
        .browser
        .session_token(&headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?
    {
        sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE token_hash=$1 AND revoked_at IS NULL")
            .bind(crate::oauth::store::hash(&session))
            .execute(&state.db)
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    }
    Ok((
        [
            ("set-cookie", state.browser.clear_session_cookie()),
            ("cache-control", "no-store".parse().unwrap()),
        ],
        Json(serde_json::json!({"logged_out": true})),
    ))
}

pub(super) async fn step_up_totp(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Json(form): Json<StepUpForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    state
        .browser
        .verify_mutation(&axum::http::Method::POST, &headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let session = state
        .browser
        .session_token(&headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?
        .ok_or(ProtocolError::OAuth(OAuthError::LoginRequired))?;
    if form.code.len() != 6 || !form.code.bytes().all(|value| value.is_ascii_digit()) {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    let expires_at = crate::mfa::grant_step_up(
        &state.db,
        &state.mfa,
        &session,
        &form.code,
        chrono::Utc::now(),
    )
    .await
    .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok((
        [("cache-control", "no-store")],
        Json(serde_json::json!({"step_up": true, "expires_at": expires_at})),
    ))
}
