use super::{AuthorizationState, OAuthError, ProtocolError, StepUpForm};
use crate::{browser::SessionProof, oauth::store};
use axum::{Extension, Json, extract::State, response::IntoResponse};

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
