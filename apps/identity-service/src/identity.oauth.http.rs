use std::sync::Arc;

use axum::{
    Extension, Form, Json, Router,
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
#[path = "identity.oauth.http.authentication_status.rs"]
mod authentication_status;
#[path = "identity.oauth.http.cors.rs"]
mod cors;
#[path = "identity.oauth.http.introspection.rs"]
mod introspection;
#[path = "identity.oauth.http.limits.rs"]
mod limits;
pub use introspection::router as introspection_router;
#[path = "identity.oauth.http.passkey_login.rs"]
mod passkey_login;
pub use passkey_login::router as passkey_login_router;
#[path = "identity.oauth.http.login_response.rs"]
mod login_response;
#[path = "identity.oauth.http.navigation.rs"]
mod navigation;
use navigation::redirect_with_result;
#[path = "identity.oauth.http.json.rs"]
mod json;
#[path = "identity.oauth.http.query.rs"]
mod query;
#[path = "identity.oauth.http.recovery.rs"]
mod recovery;
#[path = "identity.oauth.http.session.rs"]
mod session;
#[path = "identity.oauth.http.token.rs"]
mod token;
#[path = "identity.oauth.http.totp.rs"]
mod totp;
pub use recovery::router as recovery_router;
#[path = "identity.oauth.http.userinfo.rs"]
mod userinfo;
#[path = "identity.oauth.http.webauthn.rs"]
mod webauthn;
use session::{logout, step_up_totp};
use token::token;
use userinfo::userinfo;
pub use webauthn::router as webauthn_router;

use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    browser::{BrowserProof, BrowserSecurity, protect_mutation, protect_session_mutation},
    oauth::{
        clients::ClientRegistry,
        error::OAuthError,
        interactions,
        metadata::provider_metadata,
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
}

#[derive(Clone)]
struct AuthorizationState {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    browser: BrowserSecurity,
    mfa: Arc<crate::mfa_crypto::MfaCrypto>,
    limiter: crate::rate_limits::RateLimiter,
}

#[derive(Deserialize)]
struct LoginForm {
    interaction: String,
    email: String,
    password: String,
}

pub fn token_router(
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
    limiter: crate::rate_limits::RateLimiter,
) -> Router {
    let cors = cors::clients(&clients);
    Router::new()
        .route("/oauth/token", post(token))
        .route(
            "/oauth/par",
            post(par).layer(axum::extract::DefaultBodyLimit::max(8192)),
        )
        .route("/oauth/userinfo", get(userinfo).post(userinfo))
        .route_layer(axum::middleware::from_fn_with_state(
            limits::SourceLimit {
                db: db.clone(),
                limiter,
                browser: None,
            },
            limits::protect_source,
        ))
        .with_state(TokenState {
            db,
            clients,
            tokens,
        })
        .layer(cors)
}

/// Starts a direct authorization transaction and hands it to the hosted UI.
/// The browser cookie is issued here because this endpoint is reached through
/// a cross-site top-level navigation and therefore cannot use CSRF headers.
pub fn authorization_router(
    db: PgPool,
    clients: Arc<ClientRegistry>,
    browser: BrowserSecurity,
    mfa: Arc<crate::mfa_crypto::MfaCrypto>,
    limiter: crate::rate_limits::RateLimiter,
) -> Router {
    let sessions = Router::new()
        .route("/oauth/logout", post(logout))
        .route("/oauth/session/step-up/totp", post(step_up_totp))
        .route(
            "/oauth/session/totp/factors/list",
            post(totp::list).layer(axum::extract::DefaultBodyLimit::max(4096)),
        )
        .route(
            "/oauth/session/totp/factors/revoke",
            post(totp::revoke).layer(axum::extract::DefaultBodyLimit::max(4096)),
        )
        .route(
            "/oauth/session/totp/enrollment/start",
            post(totp::start).layer(axum::extract::DefaultBodyLimit::max(4096)),
        )
        .route(
            "/oauth/session/totp/enrollment/confirm",
            post(totp::confirm).layer(axum::extract::DefaultBodyLimit::max(4096)),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            browser.clone(),
            protect_session_mutation,
        ));
    Router::new()
        .route("/oauth/authorize/login", post(login))
        .route(
            "/oauth/authorize/authentication",
            post(authentication_status::read).layer(axum::extract::DefaultBodyLimit::max(4096)),
        )
        .route("/oauth/authorize/approve", post(approve))
        .route("/oauth/authorize/deny", post(deny))
        .route_layer(axum::middleware::from_fn_with_state(
            browser.clone(),
            protect_mutation,
        ))
        .merge(sessions)
        .route("/oauth/authorize", get(authorize))
        .route_layer(axum::middleware::from_fn_with_state(
            limits::SourceLimit {
                db: db.clone(),
                limiter: limiter.clone(),
                browser: Some(browser.clone()),
            },
            limits::protect_source,
        ))
        .with_state(AuthorizationState {
            db,
            clients,
            browser,
            mfa,
            limiter,
        })
}

async fn login(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<BrowserProof>,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    let authenticated = crate::oauth::login::authenticate(
        &state.db,
        &state.clients,
        &form.interaction,
        &proof,
        &form.email,
        &form.password,
        &state.limiter,
    )
    .await
    .map_err(|error| match error {
        store::StoreError::Protocol(error) => ProtocolError::OAuth(error),
        _ => ProtocolError::OAuth(OAuthError::Unavailable),
    })?;
    login_response::authenticated_response(
        &state.browser,
        &proof,
        &form.interaction,
        &authenticated.token,
        &authenticated.csrf,
    )
}

#[derive(Debug, Deserialize)]
struct InteractionForm {
    interaction: String,
}

#[derive(Deserialize)]
struct StepUpForm {
    code: String,
}

async fn approve(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<BrowserProof>,
    headers: HeaderMap,
    Json(form): Json<InteractionForm>,
) -> Result<Response, ProtocolError> {
    let code = crate::oauth::consent::approve(&state.db, &state.clients, &form.interaction, &proof)
        .await
        .map_err(protocol_store_error)?;
    Ok(navigation::interaction_result(
        &headers,
        &code.request.redirect_uri,
        "code",
        &code.code,
        &code.request.state,
    )?)
}

async fn deny(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<BrowserProof>,
    headers: HeaderMap,
    Json(form): Json<InteractionForm>,
) -> Result<Response, ProtocolError> {
    let request = crate::oauth::consent::deny(&state.db, &state.clients, &form.interaction, &proof)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(navigation::interaction_result(
        &headers,
        &request.redirect_uri,
        "error",
        "access_denied",
        &request.state,
    )?)
}

async fn authorize(
    State(state): State<AuthorizationState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Result<Response, ProtocolError> {
    let query = query::AuthorizationQuery::decode(query.as_deref().unwrap_or_default())
        .map_err(ProtocolError::OAuth)?;
    let cookie = state
        .browser
        .existing_or_new_browser_cookie(&headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let session = state
        .browser
        .session_token(&headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let handle = match query {
        query::AuthorizationQuery::Direct(input) => {
            let request = input
                .validate(&state.clients)
                .map_err(ProtocolError::OAuth)?;
            store::create_request_in(&mut tx, &request, RequestKind::Authorization).await
        }
        query::AuthorizationQuery::Par { client_id, handle } => {
            store::consume_par_in(&mut tx, &state.clients, &handle, &client_id).await
        }
    }
    .map_err(protocol_store_error)?;
    let started = interactions::begin_in(
        &mut tx,
        &state.clients,
        &handle,
        &cookie.token,
        session.as_deref(),
    )
    .await
    .map_err(protocol_store_error)?;
    tx.commit()
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
            Err(store::StoreError::Protocol(OAuthError::LoginRequired)) => {
                return Ok(redirect_with_result(
                    started.request.redirect_uri(),
                    "error",
                    "login_required",
                    &started.request.state,
                )?
                .into_response());
            }
            Err(error) => return Err(protocol_store_error(error)),
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
            "session_csrf_token": session.as_deref().filter(|_| !started.needs_login)
                .map(|session| state.browser.session_csrf_token(session, &cookie.token))
                .transpose().map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?,
            "needs_login": started.needs_login,
            "client_id": started.request.client_id(),
            "scope": started.request.scope(),
        })),
    )
        .into_response())
}

async fn par(
    State(state): State<TokenState>,
    headers: HeaderMap,
    form: Result<Form<Vec<(String, String)>>, axum::extract::rejection::FormRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let Form(fields) = form.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let query::AuthorizationQuery::Direct(mut input) =
        query::AuthorizationQuery::from_fields(fields).map_err(ProtocolError::OAuth)?
    else {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    };
    let proof = token::dpop_header(&headers)?
        .map(|proof| {
            crate::oauth::dpop::verify_token_proof(
                proof,
                "POST",
                &format!("{}/oauth/par", state.tokens.issuer().trim_end_matches('/')),
            )
        })
        .transpose()
        .map_err(ProtocolError::OAuth)?;
    if let Some(proof) = &proof {
        if input
            .dpop_jkt
            .as_deref()
            .is_some_and(|jkt| jkt != proof.thumbprint())
        {
            return Err(ProtocolError::OAuth(OAuthError::InvalidDpopProof));
        }
        input.dpop_jkt = Some(proof.thumbprint().to_owned());
    }
    let request = input
        .validate(&state.clients)
        .map_err(ProtocolError::OAuth)?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    if let Some(proof) = proof {
        proof.consume(&mut tx).await.map_err(ProtocolError::OAuth)?;
    }
    let request_uri = store::create_request_in(&mut tx, &request, RequestKind::Par)
        .await
        .map_err(protocol_store_error)?;
    tx.commit()
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    Ok((
        StatusCode::CREATED,
        [("cache-control", "no-store"), ("pragma", "no-cache")],
        Json(serde_json::json!({
            "request_uri": format!("urn:ietf:params:oauth:request_uri:{request_uri}"),
            "expires_in": 300
        })),
    ))
}

#[derive(Debug)]
enum ProtocolError {
    OAuth(OAuthError),
}

fn protocol_store_error(error: store::StoreError) -> ProtocolError {
    ProtocolError::OAuth(match error {
        store::StoreError::Protocol(error) => error,
        _ => OAuthError::Unavailable,
    })
}

impl IntoResponse for ProtocolError {
    fn into_response(self) -> axum::response::Response {
        let ProtocolError::OAuth(error) = self;
        let status = match error {
            OAuthError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            OAuthError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::BAD_REQUEST,
        };
        let mut response = (
            status,
            [("cache-control", "no-store"), ("pragma", "no-cache")],
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response();
        if let OAuthError::RateLimited(seconds) = error {
            response.headers_mut().insert("retry-after", seconds.into());
        }
        response
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
        .layer(cors::metadata())
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
