use super::{OAuthError, ProtocolError, json::Object, limits};
use crate::{
    browser::{
        BrowserSecurity, RecoveryProof, SessionProof, protect_recovery_mutation,
        protect_session_mutation,
    },
    mfa_recovery::{self, RecoveryError},
    rate_limits::{Category, RateLimiter},
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, header},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use sqlx::PgPool;
use std::sync::Arc;
use webauthn_rs::{Webauthn, prelude::RegisterPublicKeyCredential};

#[derive(Clone)]
struct RecoveryState {
    db: PgPool,
    browser: BrowserSecurity,
    server: Arc<Webauthn>,
    limiter: RateLimiter,
}

pub fn router(
    db: PgPool,
    browser: BrowserSecurity,
    server: Arc<Webauthn>,
    limiter: RateLimiter,
) -> Router {
    let regular = Router::new()
        .route("/oauth/session/recovery/codes/generate", post(generate))
        .route("/oauth/session/recovery/redeem", post(redeem))
        .layer(DefaultBodyLimit::max(4096))
        .route_layer(middleware::from_fn_with_state(
            browser.clone(),
            protect_session_mutation,
        ));
    let recovering = Router::new()
        .route(
            "/oauth/recovery/registration/options",
            post(options).layer(DefaultBodyLimit::max(4096)),
        )
        .route(
            "/oauth/recovery/registration/finish",
            post(finish).layer(DefaultBodyLimit::max(65_536)),
        )
        .route_layer(middleware::from_fn_with_state(
            browser.clone(),
            protect_recovery_mutation,
        ));
    regular
        .merge(recovering)
        .route_layer(middleware::from_fn_with_state(
            limits::SourceLimit {
                db: db.clone(),
                limiter: limiter.clone(),
                browser: Some(browser.clone()),
            },
            limits::protect_source,
        ))
        .with_state(RecoveryState {
            db,
            browser,
            server,
            limiter,
        })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Redeem {
    code: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Finish {
    ceremony_id: uuid::Uuid,
    credential: RegisterPublicKeyCredential,
    label: String,
}

fn decode<T>(body: Result<Json<Object<T>>, JsonRejection>) -> Result<T, ProtocolError> {
    body.map(|Json(Object(value))| value)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))
}
fn failure(error: RecoveryError) -> ProtocolError {
    ProtocolError::OAuth(match error {
        RecoveryError::Invalid => OAuthError::InvalidRequest,
        RecoveryError::Database(_) | RecoveryError::Serialization(_) => OAuthError::Unavailable,
    })
}

async fn quota(state: &RecoveryState, token: &str, recovering: bool) -> Result<(), ProtocolError> {
    let sql = if recovering {
        "SELECT r.principal_id FROM identity_mfa_recovery_sessions r JOIN identity_principals p ON p.id=r.principal_id WHERE r.token_hash=$1 AND r.expires_at>clock_timestamp() AND p.status='active'"
    } else {
        "SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active'"
    };
    let principal: Option<uuid::Uuid> = sqlx::query_scalar(sql)
        .bind(crate::oauth::store::hash(token))
        .fetch_optional(&state.db)
        .await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let principal = principal.ok_or(ProtocolError::OAuth(OAuthError::LoginRequired))?;
    let category = if recovering {
        Category::WebauthnAccount
    } else {
        Category::MfaAccount
    };
    crate::oauth::limits::enforce(&state.db, &state.limiter, category, &principal.to_string())
        .await
        .map_err(ProtocolError::OAuth)
}

async fn generate(
    State(state): State<RecoveryState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Empty>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    quota(&state, proof.token(), false).await?;
    let Empty {} = decode(body)?;
    let codes = mfa_recovery::generate(&state.db, proof.token())
        .await
        .map_err(failure)?;
    Ok(Json(serde_json::json!({"codes":codes.codes})))
}

async fn redeem(
    State(state): State<RecoveryState>,
    Extension(proof): Extension<SessionProof>,
    headers: HeaderMap,
    body: Result<Json<Object<Redeem>>, JsonRejection>,
) -> Result<Response, ProtocolError> {
    quota(&state, proof.token(), false).await?;
    let form = decode(body)?;
    let browser = state
        .browser
        .browser_token(&headers)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?
        .ok_or(ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let recovery = mfa_recovery::redeem(&state.db, proof.token(), &form.code)
        .await
        .map_err(failure)?;
    let csrf = state
        .browser
        .recovery_csrf_token(&recovery.token, &browser)
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let cookie = state
        .browser
        .recovery_cookie(&recovery.token)
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let mut response = Json(
        serde_json::json!({"recovery":true,"csrf_token":csrf,"expires_at":recovery.expires_at}),
    )
    .into_response();
    response.headers_mut().append(header::SET_COOKIE, cookie);
    response
        .headers_mut()
        .append(header::SET_COOKIE, state.browser.clear_session_cookie());
    Ok(response)
}

async fn options(
    State(state): State<RecoveryState>,
    Extension(proof): Extension<RecoveryProof>,
    body: Result<Json<Object<Empty>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    quota(&state, proof.token(), true).await?;
    let Empty {} = decode(body)?;
    Ok(Json(
        mfa_recovery::registration::start(&state.db, &state.server, proof.token())
            .await
            .map_err(failure)?,
    ))
}

async fn finish(
    State(state): State<RecoveryState>,
    Extension(proof): Extension<RecoveryProof>,
    body: Result<Json<Object<Finish>>, JsonRejection>,
) -> Result<Response, ProtocolError> {
    quota(&state, proof.token(), true).await?;
    let form = decode(body)?;
    let id = mfa_recovery::registration::finish(
        &state.db,
        &state.server,
        proof.token(),
        form.ceremony_id,
        &form.credential,
        &form.label,
    )
    .await
    .map_err(failure)?;
    let mut response =
        Json(serde_json::json!({"recovered":true,"credential_id":id,"must_reauthenticate":true}))
            .into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, state.browser.clear_recovery_cookie());
    response
        .headers_mut()
        .append(header::SET_COOKIE, state.browser.clear_session_cookie());
    Ok(response)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.http.recovery.tests.rs"]
mod tests;
