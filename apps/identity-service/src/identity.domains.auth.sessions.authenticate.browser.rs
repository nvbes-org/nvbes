use axum::http::HeaderMap;
use chrono::Utc;
use sqlx::PgPool;

use super::authenticate::{
    build_user_record_from_cache, enforce_cookie_theft_mitigation, get_cached_session,
};
use super::cache::{
    auth_context_from_cached_session, current_session_ttl, is_expired, refresh_last_seen,
};
use super::token;
use crate::domains::auth::{mfa, sessions::activity};
use crate::http::error::AppError;

const BROWSER_SESSION_SCOPE: &str = "openid profile email offline_access";

pub async fn authenticate_browser_session(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    browser_session_token: &str,
    headers: &HeaderMap,
) -> Result<crate::domains::auth::types::AuthContext, AppError> {
    let session_id = token::session_id(browser_session_token)?;
    let mut session = get_cached_session(db, redis, session_id)
        .await?
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found."))?;
    let expected_hash = session
        .browser_session_token_hash
        .as_deref()
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found."))?;
    if expected_hash != token::hash(browser_session_token) {
        return Err(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));
    }
    if nvbes_redis::session::is_session_revoked(redis, &session.session_id)
        .await
        .map_err(|err| {
            AppError::internal("redis_session_revocation_lookup_failed", err.to_string())
        })?
        || is_expired(&session, Utc::now())
    {
        let _ =
            nvbes_redis::session::delete_session(redis, &session.principal_id, &session.session_id)
                .await;
        return Err(AppError::unauthorized(
            "session_expired",
            "Session expired.",
        ));
    }

    let principal_id = uuid::Uuid::parse_str(&session.principal_id)
        .map_err(|_| AppError::unauthorized("session_not_found", "Session not found."))?;
    enforce_cookie_theft_mitigation(db, redis, &mut session, principal_id, session_id, headers)
        .await?;
    activity::enforce(db, redis, &mut session, principal_id, session_id, headers).await?;

    refresh_last_seen(&mut session);
    let ttl = current_session_ttl(&session);
    let _ = nvbes_redis::session::set_session(redis, &session, ttl).await;

    let user = build_user_record_from_cache(&session, db, principal_id).await?;
    if user.status != "active" {
        return Err(AppError::forbidden(
            "account_disabled",
            "This account is not active.",
        ));
    }

    Ok(auth_context_from_cached_session(
        &session,
        &user,
        mfa::has_active_factor(db, principal_id).await?,
        BROWSER_SESSION_SCOPE.to_string(),
        None,
    ))
}
