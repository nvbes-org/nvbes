use super::{AuthorizationState, OAuthError, ProtocolError, StepUpForm};
use crate::{browser::SessionProof, oauth::store};
use axum::{Extension, Json, extract::State, response::IntoResponse};

pub(super) async fn logout_context(
    State(state): State<AuthorizationState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, axum::response::Response> {
    let forbidden = |_| {
        (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"invalid_browser_request"})),
        )
            .into_response()
    };
    state
        .browser
        .verify_session_read(&headers)
        .map_err(forbidden)?;
    let session = state.browser.session_token(&headers).map_err(forbidden)?;
    let browser = state.browser.browser_token(&headers).map_err(forbidden)?;
    let csrf = match (session, browser) {
        (Some(session), Some(browser)) => {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM identity_sessions WHERE token_hash=$1 AND revoked_at IS NULL)",
            ).bind(store::hash(&session)).fetch_one(&state.db).await
                .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable).into_response())?;
            if exists {
                Some(
                    state
                        .browser
                        .session_csrf_token(&session, &browser)
                        .map_err(forbidden)?,
                )
            } else {
                None
            }
        }
        _ => None,
    };
    Ok(Json(serde_json::json!({"session_csrf_token": csrf})))
}

pub(super) async fn logout(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
) -> Result<impl IntoResponse, ProtocolError> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    crate::session_locks::session(&mut tx, proof.token())
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let session: Option<(uuid::Uuid, uuid::Uuid, bool)> = sqlx::query_as(
        "SELECT id,principal_id,revoked_at IS NOT NULL FROM identity_sessions WHERE token_hash=$1 FOR UPDATE",
    ).bind(store::hash(proof.token())).fetch_optional(&mut *tx).await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let (id, principal, revoked) =
        session.ok_or(ProtocolError::OAuth(OAuthError::LoginRequired))?;
    if !revoked {
        sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
        store::audit(&mut tx, principal, "identity.session.logged_out")
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    }
    tx.commit()
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    Ok((
        [("set-cookie", state.browser.clear_session_cookie())],
        Json(serde_json::json!({"logged_out": true})),
    ))
}

pub(super) async fn step_up_totp(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    Json(form): Json<StepUpForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    if form.code.len() != 6 || !form.code.bytes().all(|value| value.is_ascii_digit()) {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    let principal: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active'",
    ).bind(store::hash(proof.token())).fetch_optional(&state.db).await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let principal = principal.ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    crate::oauth::limits::enforce(
        &state.db,
        &state.limiter,
        crate::rate_limits::Category::MfaAccount,
        &principal.to_string(),
    )
    .await
    .map_err(ProtocolError::OAuth)?;
    let expires_at = crate::mfa::grant_step_up(
        &state.db,
        &state.mfa,
        proof.token(),
        &form.code,
        chrono::Utc::now(),
    )
    .await
    .map_err(|error| {
        let unavailable = error
            .downcast_ref::<sqlx::Error>()
            .is_some_and(|error| !matches!(error, sqlx::Error::RowNotFound));
        ProtocolError::OAuth(if unavailable {
            OAuthError::Unavailable
        } else {
            OAuthError::InvalidRequest
        })
    })?;
    Ok(Json(
        serde_json::json!({"step_up": true, "expires_at": expires_at}),
    ))
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.http.session.tests.rs"]
mod tests;
