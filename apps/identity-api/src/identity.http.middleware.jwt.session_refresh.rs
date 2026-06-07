use axum::http::{HeaderMap, HeaderValue};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::http::cookies::{
    auth_cookie, auth_cookie_name_with_user, csrf_cookie, generate_csrf_token,
};
use crate::http::error::AppError;

pub struct RefreshedCookies {
    pub session_cookie: HeaderValue,
    pub csrf_cookie: HeaderValue,
}

pub async fn authenticate_or_refresh_session(
    state: &AppState,
    headers: &HeaderMap,
    authuser: &str,
) -> Result<
    (
        crate::domains::auth::types::AuthContext,
        Option<RefreshedCookies>,
    ),
    AppError,
> {
    let token = crate::http::request::bearer_token_with_authuser(headers, authuser)?;

    match sessions::authenticate(&state.db, &state.redis, &state.jwt, &token).await {
        Ok(auth) => Ok((auth, None)),
        Err(error)
            if error.status == axum::http::StatusCode::UNAUTHORIZED
                && error.code == "token_expired" =>
        {
            refresh_expired_session(state, authuser, &token).await
        }
        Err(error) => Err(error),
    }
}

async fn refresh_expired_session(
    state: &AppState,
    authuser: &str,
    expired_token: &str,
) -> Result<
    (
        crate::domains::auth::types::AuthContext,
        Option<RefreshedCookies>,
    ),
    AppError,
> {
    let claims = state.jwt.decode_token_ignore_expiry(expired_token)?;

    let session_id = Uuid::parse_str(&claims.sid)
        .map_err(|_| AppError::unauthorized("invalid_session", "Invalid session ID"))?;
    let principal_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::unauthorized("invalid_subject", "Invalid subject"))?;

    let session = nvbes_redis::session::get_session(&state.redis, &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", &err.to_string()))?
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found"))?;

    if session.principal_id != principal_id.to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::unauthorized("session_expired", "Session expired"));
    }

    let new_token = state.jwt.generate_access_token_from_claims(&claims)?;
    rotate_session_token_hash(state, session_id, principal_id, &session, &new_token).await?;

    let auth = sessions::authenticate(&state.db, &state.redis, &state.jwt, &new_token).await?;
    let cookies = build_refreshed_cookies(state, authuser, &new_token)?;

    Ok((auth, Some(cookies)))
}

async fn rotate_session_token_hash(
    state: &AppState,
    session_id: Uuid,
    principal_id: Uuid,
    session: &nvbes_redis::session::CachedSession,
    new_token: &str,
) -> Result<(), AppError> {
    let session_ttl = (session.expires_at - chrono::Utc::now())
        .num_seconds()
        .max(1) as u64;

    if nvbes_redis::session::update_session_token_hash(
        &state.redis,
        &session_id.to_string(),
        &crate::domains::auth::password::token_hash(new_token),
        session_ttl,
    )
    .await
    .is_err()
    {
        nvbes_redis::session::delete_session(
            &state.redis,
            &principal_id.to_string(),
            &session_id.to_string(),
        )
        .await
        .map_err(|err| AppError::internal("redis_session_cache_reset_failed", &err.to_string()))?;
    }

    Ok(())
}

fn build_refreshed_cookies(
    state: &AppState,
    authuser: &str,
    new_token: &str,
) -> Result<RefreshedCookies, AppError> {
    let secure_cookie = state.config.environment != "development";
    let session_cookie_name = auth_cookie_name_with_user("session", authuser, secure_cookie);
    let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
    let session_cookie = auth_cookie(
        &session_cookie_name,
        new_token,
        session_expires_in,
        secure_cookie,
    )?;
    let csrf_token = generate_csrf_token();
    let csrf_cookie_name = auth_cookie_name_with_user("csrf_token", authuser, secure_cookie);
    let csrf_cookie = csrf_cookie(
        &csrf_cookie_name,
        &csrf_token,
        session_expires_in,
        secure_cookie,
    )?;

    Ok(RefreshedCookies {
        session_cookie,
        csrf_cookie,
    })
}
