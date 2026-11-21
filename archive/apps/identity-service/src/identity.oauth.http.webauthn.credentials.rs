use super::{Empty, WebauthnState, decode, failure};
use crate::{
    browser::SessionProof,
    oauth::http::{OAuthError, ProtocolError},
    webauthn::credentials,
};
use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rename {
    credential_id: uuid::Uuid,
    label: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Revoke {
    credential_id: uuid::Uuid,
}

pub(super) async fn list(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Empty>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    if !decode(body)?.is_empty() {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    Ok(Json(
        credentials::list(&state.db, proof.token())
            .await
            .map_err(failure)?,
    ))
}
pub(super) async fn rename(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Rename>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let form = decode(body)?;
    credentials::rename(&state.db, proof.token(), form.credential_id, &form.label)
        .await
        .map_err(failure)?;
    Ok(Json(serde_json::json!({"renamed":true})))
}
pub(super) async fn revoke(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Revoke>, JsonRejection>,
) -> Result<Response, ProtocolError> {
    let form = decode(body)?;
    match credentials::revoke(&state.db, proof.token(), form.credential_id).await {
        Err(crate::webauthn::WebauthnError::LastFactor) => {
            return Ok((
                axum::http::StatusCode::CONFLICT,
                Json(serde_json::json!({"error":"last_strong_factor"})),
            )
                .into_response());
        }
        result => result.map_err(failure)?,
    }
    Ok(Json(serde_json::json!({"revoked":true})).into_response())
}
