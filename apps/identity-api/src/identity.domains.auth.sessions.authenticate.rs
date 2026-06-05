use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::cache::{auth_context_from_cached_session, current_session_ttl, refresh_last_seen};
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::AuthContext;
use crate::domains::auth::{db as auth_db, mfa, password, sessions::cache};
use crate::http::error::AppError;

pub async fn authenticate(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    token: &str,
) -> Result<AuthContext, AppError> {
    let claims = jwt.decode_token(token, "access")?;
    if claims.token_type != "access" {
        return Err(AppError::unauthorized(
            "invalid_token_type",
            "Access token is required.",
        ));
    }

    let principal_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::unauthorized("invalid_subject", &format!("{}", e)))?;
    let session_id = Uuid::parse_str(&claims.sid)
        .map_err(|e| AppError::unauthorized("invalid_session", &format!("{}", e)))?;
    let token_hash = password::token_hash(token);

    let mut session = get_cached_session(redis, session_id)
        .await?
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found."))?;
    if session.principal_id != principal_id.to_string() {
        return Err(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));
    }
    if session.token_hash != token_hash {
        return Err(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));
    }
    if nvbes_redis::session::is_session_revoked(redis, &session.session_id)
        .await
        .map_err(|err| {
            AppError::internal("redis_session_revocation_lookup_failed", &err.to_string())
        })?
        || session.expires_at <= Utc::now()
    {
        let _ =
            nvbes_redis::session::delete_session(redis, &session.principal_id, &session.session_id)
                .await;
        return Err(AppError::unauthorized(
            "session_expired",
            "Session expired.",
        ));
    }

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
        claims.scope,
        claims.cnf.map(|c| c.jkt),
    ))
}

async fn get_cached_session(
    redis: &nvbes_redis::RedisPool,
    session_id: Uuid,
) -> Result<Option<cache::CachedSession>, AppError> {
    nvbes_redis::session::get_session(redis, &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_cache_read_failed", &err.to_string()))
}

async fn build_user_record_from_cache(
    session: &cache::CachedSession,
    db: &PgPool,
    principal_id: Uuid,
) -> Result<auth_db::UserRecord, AppError> {
    let user = auth_db::fetch_user_record(db, principal_id).await?;
    if session.principal_id != user.principal_id.to_string() {
        return Err(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));
    }
    Ok(user)
}
