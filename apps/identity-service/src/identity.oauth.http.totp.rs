use super::{AuthorizationState, OAuthError, ProtocolError};
use crate::{
    browser::SessionProof,
    totp::{self, TotpError},
};
use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    response::IntoResponse,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Start {}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirm {
    factor_id: uuid::Uuid,
    code: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Revoke {
    factor_id: uuid::Uuid,
}

use super::json::Object;

fn failure(error: TotpError) -> ProtocolError {
    ProtocolError::OAuth(match error {
        TotpError::Crypto | TotpError::Database(_) => OAuthError::Unavailable,
        _ => OAuthError::InvalidRequest,
    })
}

async fn quota(state: &AuthorizationState, proof: &SessionProof) -> Result<(), ProtocolError> {
    let principal: Option<uuid::Uuid> = sqlx::query_scalar("SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active'")
        .bind(crate::auth::hash_token(proof.token())).fetch_optional(&state.db).await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let principal = principal.ok_or(ProtocolError::OAuth(OAuthError::LoginRequired))?;
    crate::oauth::limits::enforce(
        &state.db,
        &state.limiter,
        crate::rate_limits::Category::MfaAccount,
        &principal.to_string(),
    )
    .await
    .map_err(ProtocolError::OAuth)
}

pub(super) async fn start(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Start>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    quota(&state, &proof).await?;
    let Json(Object(Start {})) =
        body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(Json(
        totp::start(&state.db, &state.mfa, proof.token())
            .await
            .map_err(failure)?,
    ))
}

pub(super) async fn confirm(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Confirm>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    quota(&state, &proof).await?;
    let Json(Object(form)) = body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let expires = totp::confirm(
        &state.db,
        &state.mfa,
        proof.token(),
        form.factor_id,
        &form.code,
    )
    .await
    .map_err(failure)?;
    Ok(Json(
        serde_json::json!({"enrolled":true,"step_up":true,"expires_at":expires}),
    ))
}

pub(super) async fn list(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Start>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let Json(Object(Start {})) =
        body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(Json(
        totp::management::list(&state.db, proof.token())
            .await
            .map_err(failure)?,
    ))
}

pub(super) async fn revoke(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Revoke>>, JsonRejection>,
) -> Result<axum::response::Response, ProtocolError> {
    quota(&state, &proof).await?;
    let Json(Object(form)) = body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    match totp::management::revoke(&state.db, proof.token(), form.factor_id).await {
        Err(TotpError::LastFactor) => Ok((
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"last_strong_factor"})),
        )
            .into_response()),
        result => {
            result.map_err(failure)?;
            Ok(Json(serde_json::json!({"revoked":true,"sessions_revoked":true})).into_response())
        }
    }
}
