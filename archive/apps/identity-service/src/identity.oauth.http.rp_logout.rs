use super::{OAuthError, ProtocolError, limits};
use crate::{
    browser::{BrowserSecurity, SessionProof, protect_session_mutation},
    oauth::{clients::ClientRegistry, logout_request::LogoutRequest, store},
    tokens::TokenService,
};
use axum::{
    Extension, Form, Json, Router,
    extract::{DefaultBodyLimit, Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
struct LogoutState {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
    browser: BrowserSecurity,
}

pub fn router(
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
    browser: BrowserSecurity,
    limiter: crate::rate_limits::RateLimiter,
) -> Router {
    let protected = Router::new()
        .route("/oauth/logout/prepare", post(prepare))
        .route("/oauth/logout/confirm", post(confirm))
        .route_layer(axum::middleware::from_fn_with_state(
            browser.clone(),
            protect_session_mutation,
        ));
    Router::new()
        .route("/oauth/end-session", get(start_get).post(start_post))
        .merge(protected)
        .layer(DefaultBodyLimit::max(32_768))
        .route_layer(axum::middleware::from_fn_with_state(
            limits::SourceLimit {
                db: db.clone(),
                limiter,
                browser: Some(browser.clone()),
            },
            limits::protect_source,
        ))
        .with_state(LogoutState {
            db,
            clients,
            tokens,
            browser,
        })
}

async fn start_get(
    State(state): State<LogoutState>,
    query: Result<Query<Vec<(String, String)>>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ProtocolError> {
    start(&state, query.map_err(|_| invalid())?.0)
}

async fn start_post(
    State(state): State<LogoutState>,
    form: Result<Form<Vec<(String, String)>>, axum::extract::rejection::FormRejection>,
) -> Result<Response, ProtocolError> {
    start(&state, form.map_err(|_| invalid())?.0)
}

fn start(state: &LogoutState, fields: Vec<(String, String)>) -> Result<Response, ProtocolError> {
    let request = LogoutRequest::from_fields(fields, &state.clients, &state.tokens)
        .map_err(ProtocolError::OAuth)?;
    let ticket = state.tokens.logout_ticket(request).map_err(|_| invalid())?;
    // A fragment never reaches the hosted web server or a Referer. The page
    // removes it from history before any request and keeps it only in memory.
    Ok(Redirect::to(&format!("/logout#request={ticket}")).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TicketForm {
    request: String,
}

async fn prepare(
    State(state): State<LogoutState>,
    Extension(proof): Extension<SessionProof>,
    Json(form): Json<TicketForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    let request = state
        .tokens
        .read_logout_ticket(&form.request, &state.clients)
        .map_err(|_| invalid())?;
    let session: Option<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
        "SELECT id,principal_id FROM identity_sessions WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp()-interval '1 hour'"
    ).bind(store::hash(proof.token())).fetch_optional(&state.db).await.map_err(|_| unavailable())?;
    bind_session(&request, session.ok_or_else(invalid)?)?;
    Ok(Json(serde_json::json!({"prepared":true})))
}

async fn confirm(
    State(state): State<LogoutState>,
    Extension(proof): Extension<SessionProof>,
    Json(form): Json<TicketForm>,
) -> Result<impl IntoResponse, ProtocolError> {
    let request = state
        .tokens
        .read_logout_ticket(&form.request, &state.clients)
        .map_err(|_| invalid())?;
    let mut tx = state.db.begin().await.map_err(|_| unavailable())?;
    crate::session_locks::session(&mut tx, proof.token())
        .await
        .map_err(|_| unavailable())?;
    let session: Option<(uuid::Uuid,uuid::Uuid,bool)> = sqlx::query_as(
        "SELECT id,principal_id,revoked_at IS NOT NULL FROM identity_sessions WHERE token_hash=$1 AND expires_at>clock_timestamp()-interval '1 hour' FOR UPDATE"
    ).bind(store::hash(proof.token())).fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    let (id, principal, revoked) = session.ok_or_else(invalid)?;
    bind_session(&request, (id, principal))?;
    // Validate the destination before committing any change.
    let redirect_uri = request.return_uri().map_err(ProtocolError::OAuth)?;
    if !revoked {
        sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|_| unavailable())?;
        store::audit(&mut tx, principal, "identity.session.logged_out")
            .await
            .map_err(|_| unavailable())?;
    }
    tx.commit().await.map_err(|_| unavailable())?;
    Ok((
        [("set-cookie", state.browser.clear_session_cookie())],
        Json(serde_json::json!({"logged_out":true,"redirect_uri":redirect_uri})),
    ))
}

fn bind_session(
    request: &LogoutRequest,
    (id, principal): (uuid::Uuid, uuid::Uuid),
) -> Result<(), ProtocolError> {
    if request
        .hint
        .as_ref()
        .is_some_and(|hint| hint.session_id != id || hint.principal_id != principal)
    {
        return Err(invalid());
    }
    Ok(())
}
fn invalid() -> ProtocolError {
    ProtocolError::OAuth(OAuthError::InvalidRequest)
}
fn unavailable() -> ProtocolError {
    ProtocolError::OAuth(OAuthError::Unavailable)
}
