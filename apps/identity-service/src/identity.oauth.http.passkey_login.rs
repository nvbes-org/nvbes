use super::{
    OAuthError, ProtocolError, limits, login_response::authenticated_response, protocol_store_error,
};
use crate::{
    browser::{BrowserProof, BrowserSecurity, protect_mutation},
    oauth::{clients::ClientRegistry, passkey_login},
    rate_limits::RateLimiter,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use sqlx::PgPool;
use std::sync::Arc;
use webauthn_rs::{Webauthn, prelude::PublicKeyCredential};

#[derive(Clone)]
struct LoginState {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    server: Arc<Webauthn>,
    browser: BrowserSecurity,
}

pub fn router(
    db: PgPool,
    clients: Arc<ClientRegistry>,
    browser: BrowserSecurity,
    server: Arc<Webauthn>,
    limiter: RateLimiter,
) -> Router {
    Router::new()
        .route("/oauth/authorize/passkey/options", post(options))
        .route("/oauth/authorize/passkey/finish", post(finish))
        .layer(DefaultBodyLimit::max(65_536))
        .route_layer(middleware::from_fn_with_state(
            browser.clone(),
            protect_mutation,
        ))
        .route_layer(middleware::from_fn_with_state(
            limits::SourceLimit {
                db: db.clone(),
                limiter,
                browser: Some(browser.clone()),
            },
            limits::protect_source,
        ))
        .with_state(LoginState {
            db,
            clients,
            server,
            browser,
        })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    interaction: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Finish {
    interaction: String,
    ceremony_id: uuid::Uuid,
    credential: PublicKeyCredential,
}

async fn options(
    State(state): State<LoginState>,
    Extension(proof): Extension<BrowserProof>,
    body: Result<Json<Options>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let Json(form) = body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(Json(
        passkey_login::start(
            &state.db,
            &state.server,
            &state.clients,
            &form.interaction,
            &proof,
        )
        .await
        .map_err(protocol_store_error)?,
    ))
}

async fn finish(
    State(state): State<LoginState>,
    Extension(proof): Extension<BrowserProof>,
    body: Result<Json<Finish>, JsonRejection>,
) -> Result<Response, ProtocolError> {
    let Json(form) = body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let authenticated = passkey_login::finish(
        &state.db,
        &state.server,
        &state.clients,
        &form.interaction,
        &proof,
        form.ceremony_id,
        &form.credential,
    )
    .await
    .map_err(protocol_store_error)?;
    authenticated_response(
        &state.browser,
        &proof,
        &form.interaction,
        &authenticated.token,
        &authenticated.csrf,
    )
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.http.passkey_login.tests.rs"]
mod tests;
