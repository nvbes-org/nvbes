use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
#[path = "identity.oauth.http.session.rs"]
mod session;
#[path = "identity.oauth.http.token.rs"]
mod token;
use session::{logout, step_up_totp};
use token::token;

use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    browser::BrowserSecurity,
    oauth::{
        clients::ClientRegistry,
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
    mfa: Arc<crate::mfa_crypto::MfaCrypto>,
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    interaction: String,
    email: String,
    password: String,
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
        .route("/oauth/userinfo", get(userinfo))
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
    mfa: Arc<crate::mfa_crypto::MfaCrypto>,
) -> Router {
    Router::new()
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/authorize/login", post(login))
        .route("/oauth/authorize/approve", post(approve))
        .route("/oauth/authorize/deny", post(deny))
        .route("/oauth/logout", post(logout))
        .route("/oauth/session/step-up/totp", post(step_up_totp))
        .with_state(AuthorizationState {
            db,
            clients,
            browser,
            mfa,
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

#[derive(Debug, Deserialize)]
struct StepUpForm {
    code: String,
}

async fn approve(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Json(form): Json<InteractionForm>,
) -> Result<Response, ProtocolError> {
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
) -> Result<Response, ProtocolError> {
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
) -> Result<Response, ProtocolError> {
    let mut url = reqwest::Url::parse(redirect_uri)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    url.query_pairs_mut()
        .append_pair(key, value)
        .append_pair("state", state);
    // RFC 9700 section 4.12: never forward a hosted POST body to the client.
    Ok((
        [
            ("cache-control", "no-store"),
            ("pragma", "no-cache"),
            ("referrer-policy", "no-referrer"),
        ],
        Redirect::to(url.as_str()),
    )
        .into_response())
}

async fn authorize(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, ProtocolError> {
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
    if started.request.prompt.as_deref() == Some("none") {
        let Some(session) = session.as_deref() else {
            return Ok(redirect_with_result(
                started.request.redirect_uri(),
                "error",
                "login_required",
                &started.request.state,
            )?
            .into_response());
        };
        match crate::oauth::consent::silent(
            &state.db,
            &state.clients,
            &handle,
            &cookie.token,
            Some(session),
        )
        .await
        {
            Ok(code) => {
                return Ok(redirect_with_result(
                    code.request.redirect_uri(),
                    "code",
                    &code.code,
                    &code.request.state,
                )?
                .into_response());
            }
            Err(crate::oauth::store::StoreError::Protocol(OAuthError::ConsentRequired)) => {
                return Ok(redirect_with_result(
                    started.request.redirect_uri(),
                    "error",
                    "consent_required",
                    &started.request.state,
                )?
                .into_response());
            }
            Err(_) => {
                return Ok(redirect_with_result(
                    started.request.redirect_uri(),
                    "error",
                    "login_required",
                    &started.request.state,
                )?
                .into_response());
            }
        }
    }
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
    )
        .into_response())
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

async fn userinfo(
    State(state): State<TokenState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ProtocolError> {
    let value = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.len() > 7 && value[..7].eq_ignore_ascii_case("bearer "))
        .map(|value| value[7..].trim())
        .filter(|value| !value.is_empty())
        .ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let claims = state
        .tokens
        .introspect(
            &state.db,
            &state.clients,
            value,
            crate::tokens_policy::ACCOUNT_AUDIENCE,
        )
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?
        .ok_or(ProtocolError::OAuth(OAuthError::InvalidGrant))?;
    let principal_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidGrant))?;
    let email: Option<String> = if claims.scope.split(' ').any(|scope| scope == "email") {
        sqlx::query_scalar("SELECT normalized_value FROM identity_login_identifiers WHERE principal_id=$1 AND kind='email' AND verified_at IS NOT NULL ORDER BY created_at LIMIT 1")
            .bind(principal_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?
    } else {
        None
    };
    let mut response = serde_json::json!({"sub": claims.sub});
    if let Some(email) = email {
        response["email"] = serde_json::Value::String(email);
        response["email_verified"] = serde_json::Value::Bool(true);
    }
    Ok(([("cache-control", "no-store")], Json(response)))
}

#[derive(Debug)]
enum ProtocolError {
    OAuth(OAuthError),
}

impl IntoResponse for ProtocolError {
    fn into_response(self) -> axum::response::Response {
        let ProtocolError::OAuth(error) = self;
        (
            if error == OAuthError::Unavailable {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::BAD_REQUEST
            },
            [("cache-control", "no-store"), ("pragma", "no-cache")],
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
