use std::sync::Arc;

use axum::{
    Form, Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    oauth::{
        clients::ClientRegistry,
        codes::CodeExchange,
        dpop::verify_and_consume_code_proof,
        error::OAuthError,
        metadata::provider_metadata,
        request::AuthorizationInput,
        store::{self, RequestKind},
    },
    tokens::TokenService,
    tokens_claims::JsonWebKeySet,
};

#[derive(Clone)]
struct PublicProtocolState {
    issuer: String,
    tokens: Arc<TokenService>,
}

#[derive(Clone)]
struct TokenState {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
    endpoint: String,
}

#[derive(Debug, Deserialize)]
struct TokenForm {
    grant_type: String,
    code: String,
    client_id: String,
    redirect_uri: String,
    code_verifier: String,
}

pub fn token_router(
    issuer: &str,
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
) -> Router {
    Router::new()
        .route("/oauth/token", post(token))
        .route("/oauth/par", post(par))
        .with_state(TokenState {
            db,
            clients,
            tokens,
            endpoint: format!("{issuer}oauth/token"),
        })
}

async fn par(
    State(state): State<TokenState>,
    Form(input): Form<AuthorizationInput>,
) -> Result<impl IntoResponse, ProtocolError> {
    let request = input
        .validate(&state.clients)
        .map_err(ProtocolError::OAuth)?;
    let request_uri = store::create_request(&state.db, &request, RequestKind::Par)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    Ok((
        [("cache-control", "no-store"), ("pragma", "no-cache")],
        Json(serde_json::json!({
            "request_uri": format!("urn:ietf:params:oauth:request_uri:{request_uri}"),
            "expires_in": 300
        })),
    ))
}

async fn token(
    State(state): State<TokenState>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    if form.grant_type != "authorization_code"
        || form.code.is_empty()
        || form.client_id.is_empty()
        || form.redirect_uri.is_empty()
        || form.code_verifier.is_empty()
    {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    let verified_dpop_jkt = match headers.get("dpop").and_then(|value| value.to_str().ok()) {
        Some(proof) => Some(
            verify_and_consume_code_proof(&state.db, proof, "POST", &state.endpoint)
                .await
                .map_err(ProtocolError::OAuth)?,
        ),
        None => None,
    };
    let grant = crate::oauth::codes::exchange(
        &state.db,
        &state.clients,
        CodeExchange {
            code: &form.code,
            client_id: &form.client_id,
            redirect_uri: &form.redirect_uri,
            verifier: &form.code_verifier,
            verified_dpop_jkt: verified_dpop_jkt.as_deref(),
        },
    )
    .await
    .map_err(|error| match error {
        crate::oauth::store::StoreError::Protocol(error) => ProtocolError::OAuth(error),
        _ => ProtocolError::OAuth(OAuthError::Unavailable),
    })?;
    Ok(Json(
        state
            .tokens
            .issue_grant(&state.db, &state.clients, &grant)
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?,
    ))
}

#[derive(Debug)]
enum ProtocolError {
    OAuth(OAuthError),
}

impl IntoResponse for ProtocolError {
    fn into_response(self) -> axum::response::Response {
        let ProtocolError::OAuth(error) = self;
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response()
    }
}

/// Safe read-only protocol endpoints. Authorization and token mutations are
/// mounted separately once their hosted-login boundary is enabled.
pub fn router(issuer: &str, tokens: Arc<TokenService>) -> Router {
    let state = PublicProtocolState {
        issuer: issuer.to_owned(),
        tokens,
    };
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/oauth/jwks", get(jwks))
        .with_state(state)
}

async fn discovery(State(state): State<PublicProtocolState>) -> Json<impl serde::Serialize> {
    Json(provider_metadata(&state.issuer))
}

async fn jwks(State(state): State<PublicProtocolState>) -> Json<JsonWebKeySet> {
    Json(state.tokens.jwks())
}

#[cfg(test)]
#[path = "identity.oauth.http.tests.rs"]
mod tests;
