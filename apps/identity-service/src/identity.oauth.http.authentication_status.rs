use super::{AuthorizationState, ProtocolError, json::Object, protocol_store_error};
use crate::browser::BrowserProof;
use axum::{Extension, Json, extract::State, response::IntoResponse};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    interaction: String,
}

pub(super) async fn read(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<BrowserProof>,
    body: Result<Json<Object<Input>>, axum::extract::rejection::JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let Json(Object(input)) =
        body.map_err(|_| ProtocolError::OAuth(super::OAuthError::InvalidRequest))?;
    let status = crate::oauth::authentication_status::read(
        &state.db,
        &state.clients,
        &input.interaction,
        &proof,
    )
    .await
    .map_err(protocol_store_error)?;
    Ok((
        [("cache-control", "no-store"), ("pragma", "no-cache")],
        Json(status),
    ))
}
