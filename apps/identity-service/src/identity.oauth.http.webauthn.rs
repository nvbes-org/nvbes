use super::{OAuthError, ProtocolError, limits};
use crate::{
    browser::{BrowserSecurity, SessionProof, protect_session_mutation},
    rate_limits::{Category, RateLimiter},
    webauthn::{WebauthnError, registration, step_up},
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Request, State, rejection::JsonRejection},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use sqlx::PgPool;
use std::sync::Arc;
use webauthn_rs::{
    Webauthn,
    prelude::{PublicKeyCredential, RegisterPublicKeyCredential},
};

#[derive(Clone)]
struct WebauthnState {
    db: PgPool,
    server: Arc<Webauthn>,
    limiter: RateLimiter,
}

pub fn router(
    db: PgPool,
    browser: BrowserSecurity,
    server: Arc<Webauthn>,
    limiter: RateLimiter,
) -> Router {
    let state = WebauthnState {
        db: db.clone(),
        server,
        limiter: limiter.clone(),
    };
    Router::new()
        .route(
            "/oauth/session/webauthn/registration/options",
            post(registration_options),
        )
        .route(
            "/oauth/session/webauthn/registration/finish",
            post(registration_finish),
        )
        .route(
            "/oauth/session/webauthn/step-up/options",
            post(step_up_options),
        )
        .route(
            "/oauth/session/webauthn/step-up/finish",
            post(step_up_finish),
        )
        .layer(DefaultBodyLimit::max(65_536))
        .route_layer(middleware::from_fn_with_state(state.clone(), account_limit))
        .route_layer(middleware::from_fn_with_state(
            browser.clone(),
            protect_session_mutation,
        ))
        .route_layer(middleware::from_fn_with_state(
            limits::SourceLimit {
                db,
                limiter,
                browser: Some(browser),
            },
            limits::protect_source,
        ))
        .with_state(state)
}

async fn account_limit(
    State(state): State<WebauthnState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(proof) = request.extensions().get::<SessionProof>() else {
        return ProtocolError::OAuth(OAuthError::InvalidRequest).into_response();
    };
    let principal: Result<Option<uuid::Uuid>, _> = sqlx::query_scalar("SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active'")
        .bind(crate::oauth::store::hash(proof.token())).fetch_optional(&state.db).await;
    let principal = match principal {
        Ok(Some(id)) => id,
        Ok(None) => return ProtocolError::OAuth(OAuthError::LoginRequired).into_response(),
        Err(_) => return ProtocolError::OAuth(OAuthError::Unavailable).into_response(),
    };
    match crate::oauth::limits::enforce(
        &state.db,
        &state.limiter,
        Category::MfaAccount,
        &principal.to_string(),
    )
    .await
    {
        Ok(()) => next.run(request).await,
        Err(error) => ProtocolError::OAuth(error).into_response(),
    }
}

type Empty = std::collections::BTreeMap<String, serde_json::Value>;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrationFinish {
    ceremony_id: uuid::Uuid,
    credential: RegisterPublicKeyCredential,
    label: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StepUpFinish {
    ceremony_id: uuid::Uuid,
    credential: PublicKeyCredential,
}

fn decode<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ProtocolError> {
    body.map(|Json(value)| value)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))
}
fn failure(error: WebauthnError) -> ProtocolError {
    ProtocolError::OAuth(match error {
        WebauthnError::Database(_) | WebauthnError::Serialization(_) => OAuthError::Unavailable,
        WebauthnError::InvalidSession => OAuthError::LoginRequired,
        _ => OAuthError::InvalidRequest,
    })
}

async fn registration_options(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Empty>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    if !decode(body)?.is_empty() {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    Ok(Json(
        registration::start(&state.db, &state.server, proof.token())
            .await
            .map_err(failure)?,
    ))
}
async fn registration_finish(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<RegistrationFinish>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let form = decode(body)?;
    let id = registration::finish(
        &state.db,
        &state.server,
        proof.token(),
        form.ceremony_id,
        &form.credential,
        &form.label,
    )
    .await
    .map_err(failure)?;
    Ok(Json(serde_json::json!({"credential_id":id})))
}
async fn step_up_options(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Empty>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    if !decode(body)?.is_empty() {
        return Err(ProtocolError::OAuth(OAuthError::InvalidRequest));
    }
    Ok(Json(
        step_up::start(&state.db, &state.server, proof.token())
            .await
            .map_err(failure)?,
    ))
}
async fn step_up_finish(
    State(state): State<WebauthnState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<StepUpFinish>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    let form = decode(body)?;
    let expires = step_up::finish(
        &state.db,
        &state.server,
        proof.token(),
        form.ceremony_id,
        &form.credential,
    )
    .await
    .map_err(failure)?;
    Ok(Json(
        serde_json::json!({"step_up":true,"expires_at":expires}),
    ))
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.http.webauthn.tests.rs"]
mod tests;
