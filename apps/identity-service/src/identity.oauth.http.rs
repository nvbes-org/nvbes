use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    browser::BrowserSecurity,
    oauth::{
        clients::ClientRegistry,
        codes::CodeExchange,
        dpop::verify_and_consume_code_proof,
        error::OAuthError,
        interactions,
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

#[derive(Clone)]
struct AuthorizationState {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    browser: BrowserSecurity,
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    interaction: String,
    email: String,
    password: String,
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

/// Starts a direct authorization transaction and hands it to the hosted UI.
/// The browser cookie is issued here because this endpoint is reached through
/// a cross-site top-level navigation and therefore cannot use CSRF headers.
pub fn authorization_router(
    db: PgPool,
    clients: Arc<ClientRegistry>,
    browser: BrowserSecurity,
) -> Router {
    Router::new()
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/authorize/login", post(login))
        .route("/oauth/authorize/approve", post(approve))
        .route("/oauth/authorize/deny", post(deny))
        .with_state(AuthorizationState {
            db,
            clients,
            browser,
        })
}

async fn login(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    let proof = state
        .browser
        .verify_mutation(&axum::http::Method::POST, &headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let session = crate::auth::authenticate(&state.db, &form.email, &form.password)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let csrf = interactions::attach_authenticated_session(
        &state.db,
        &state.clients,
        &form.interaction,
        &proof,
        &session,
    )
    .await
    .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let session_cookie = state
        .browser
        .session_cookie(&session)
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    Ok((
        [
            ("set-cookie", session_cookie),
            ("cache-control", "no-store".parse().unwrap()),
        ],
        Json(serde_json::json!({
            "interaction": form.interaction,
            "csrf_token": csrf
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct InteractionForm {
    interaction: String,
}

async fn approve(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Json(form): Json<InteractionForm>,
) -> Result<Redirect, ProtocolError> {
    let proof = state
        .browser
        .verify_mutation(&axum::http::Method::POST, &headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let code = crate::oauth::consent::approve(&state.db, &state.clients, &form.interaction, &proof)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(redirect_with_result(
        &code.request.redirect_uri,
        "code",
        &code.code,
        &code.request.state,
    )?)
}

async fn deny(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Json(form): Json<InteractionForm>,
) -> Result<Redirect, ProtocolError> {
    let proof = state
        .browser
        .verify_mutation(&axum::http::Method::POST, &headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let request = crate::oauth::consent::deny(&state.db, &state.clients, &form.interaction, &proof)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(redirect_with_result(
        &request.redirect_uri,
        "error",
        "access_denied",
        &request.state,
    )?)
}

fn redirect_with_result(
    redirect_uri: &str,
    key: &str,
    value: &str,
    state: &str,
) -> Result<Redirect, ProtocolError> {
    let mut url = reqwest::Url::parse(redirect_uri)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    url.query_pairs_mut()
        .append_pair(key, value)
        .append_pair("state", state);
    Ok(Redirect::temporary(url.as_str()))
}

async fn authorize(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ProtocolError> {
    let client_id = query
        .get("client_id")
        .ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let request = if let Some(uri) = query.get("request_uri") {
        let prefix = "urn:ietf:params:oauth:request_uri:";
        let handle = uri
            .strip_prefix(prefix)
            .ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))?;
        let handle = store::consume_par(&state.db, &state.clients, handle, client_id)
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
        store::load_request(&state.db, &state.clients, &handle)
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?
    } else {
        let value = serde_json::to_value(&query)
            .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
        let input: AuthorizationInput = serde_json::from_value(value)
            .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
        input
            .validate(&state.clients)
            .map_err(ProtocolError::OAuth)?
    };
    let handle = store::create_request(&state.db, &request, RequestKind::Authorization)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let cookie = state.browser.browser_cookie();
    let session = state
        .browser
        .session_token(&headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let started = interactions::begin(
        &state.db,
        &state.clients,
        &handle,
        &cookie.token,
        session.as_deref(),
    )
    .await
    .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    Ok((
        [
            ("set-cookie", cookie.header),
            ("cache-control", "no-store".parse().unwrap()),
        ],
        Json(serde_json::json!({
            "interaction": handle,
            "csrf_token": started.csrf_token,
            "needs_login": started.needs_login,
            "client_id": started.request.client_id(),
            "scope": started.request.scope(),
        })),
    ))
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
