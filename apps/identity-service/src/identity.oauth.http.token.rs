use super::{ProtocolError, TokenState};
use crate::{
    oauth::{
        codes::{self, CodeExchange},
        dpop::verify_and_consume_code_proof,
        error::OAuthError,
        store::StoreError,
    },
    tokens::RefreshRequest,
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
    let verified_dpop_jkt = match proof {
        Some(proof) => Some(
            verify_and_consume_code_proof(&state.db, proof, "POST", &state.endpoint)
                .await
                .map_err(ProtocolError::OAuth)?,
        ),
        None => None,
    };
    let grant = codes::exchange(
        &state.db,
        &state.clients,
        CodeExchange {
            code,
            client_id,
            redirect_uri,
            verifier,
            verified_dpop_jkt: verified_dpop_jkt.as_deref(),
        },
    )
    .await
    .map_err(|error| match error {
        StoreError::Protocol(error) => ProtocolError::OAuth(error),
        _ => ProtocolError::OAuth(OAuthError::Unavailable),
    })?;
    let response = state
        .tokens
        .issue_grant(&state.db, &state.clients, &grant)
        .await
        .map_err(token_error)?;
    Ok(token_response(response))
}

fn required(value: Option<&str>) -> Result<&str, ProtocolError> {
    value
        .filter(|value| !value.is_empty())
        .ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))
}

fn dpop_header(headers: &HeaderMap) -> Result<Option<&str>, ProtocolError> {
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
