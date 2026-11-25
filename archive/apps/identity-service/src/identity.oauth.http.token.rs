use super::{ProtocolError, TokenState};
use crate::{
    oauth::error::OAuthError,
    tokens::{AuthorizationCodeRequest, RefreshRequest},
    tokens_claims::TokenSet,
    tokens_error::TokenError,
};
use axum::{
    Form, Json,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct TokenForm {
    grant_type: String,
    code: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

pub(super) async fn token(
    State(state): State<TokenState>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Result<Response, ProtocolError> {
    let client_id = required(form.client_id.as_deref())?;
    let proof = dpop_header(&headers)?;
    if form.grant_type == "refresh_token" {
        let response = state
            .tokens
            .refresh(
                &state.db,
                &state.clients,
                RefreshRequest {
                    refresh_token: required(form.refresh_token.as_deref())?,
                    client_id,
                    dpop_proof: proof,
                },
            )
            .await
            .map_err(token_error)?;
        return Ok(token_response(response));
    }
    if form.grant_type != "authorization_code" {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    let code = required(form.code.as_deref())?;
    let redirect_uri = required(form.redirect_uri.as_deref())?;
    let verifier = required(form.code_verifier.as_deref())?;
    let response = state
        .tokens
        .exchange_code(
            &state.db,
            &state.clients,
            AuthorizationCodeRequest {
                code,
                client_id,
                redirect_uri,
                verifier,
                dpop_proof: proof,
            },
        )
        .await
        .map_err(token_error)?;
    Ok(token_response(response))
}

fn required(value: Option<&str>) -> Result<&str, ProtocolError> {
    value
        .filter(|value| !value.is_empty())
        .ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))
}

pub(super) fn dpop_header(headers: &HeaderMap) -> Result<Option<&str>, ProtocolError> {
    let mut values = headers.get_all("dpop").iter();
    let value = values.next();
    if values.next().is_some() {
        return Err(ProtocolError::OAuth(OAuthError::InvalidDpopProof));
    }
    value
        .map(|value| {
            value
                .to_str()
                .ok()
                .filter(|value| !value.is_empty())
                .ok_or(ProtocolError::OAuth(OAuthError::InvalidDpopProof))
        })
        .transpose()
}

fn token_error(error: TokenError) -> ProtocolError {
    ProtocolError::OAuth(match error {
        TokenError::Authorization(OAuthError::InvalidDpopProof) => OAuthError::InvalidDpopProof,
        TokenError::Authorization(OAuthError::InvalidClient) => OAuthError::InvalidClient,
        TokenError::Database(_) | TokenError::Authorization(OAuthError::Unavailable) => {
            OAuthError::Unavailable
        }
        _ => OAuthError::InvalidGrant,
    })
}

fn token_response(tokens: TokenSet) -> Response {
    (
        [("cache-control", "no-store"), ("pragma", "no-cache")],
        Json(tokens),
    )
        .into_response()
}
