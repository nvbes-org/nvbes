use chrono::Utc;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use super::types::*;
use crate::domains::auth::sessions::cache::session_view_from_cached_session;
use crate::http::error::AppError;

pub async fn logout(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    session_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let _ = db;
    nvbes_redis::refresh_token::revoke_session_refresh_tokens(redis, user_id, session_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;

    nvbes_redis::session::delete_session(redis, &user_id.to_string(), &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;
    Ok(())
}

pub async fn list(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    current_session_id: Uuid,
) -> Result<SessionsResult, AppError> {
    let session_ids = nvbes_redis::session::list_user_sessions(redis, &user_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_list_failed", &err.to_string()))?;
    let mut sessions = Vec::new();

    for session_id in session_ids {
        let Ok(Some(session)) = nvbes_redis::session::get_session(redis, &session_id).await else {
            continue;
        };
        if session.principal_id != user_id.to_string()
            || session.revoked_at.is_some()
            || session.expires_at <= Utc::now()
        {
            continue;
        }
        sessions.push(session_view_from_cached_session(
            &session,
            session.session_id == current_session_id.to_string(),
        ));
    }

    sessions.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    Ok(SessionsResult { sessions })
}

pub async fn revoke(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    let _ =
        nvbes_redis::session::delete_session(redis, &user_id.to_string(), &session_id.to_string())
            .await;
    let _ = db;
    nvbes_redis::refresh_token::revoke_session_refresh_tokens(redis, user_id, session_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;

    Ok(())
}

pub async fn revoke_all_others(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    current_session_id: Uuid,
) -> Result<(), AppError> {
    nvbes_redis::session::clear_user_sessions_except(
        redis,
        &user_id.to_string(),
        &current_session_id.to_string(),
    )
    .await
    .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;
    let _ = db;
    nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, user_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;

    Ok(())
}

pub async fn fetch_view(
    redis: &nvbes_redis::RedisPool,
    session_id: Uuid,
    current: bool,
) -> Result<SessionView, AppError> {
    let session = nvbes_redis::session::get_session(redis, &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", &err.to_string()))?
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found."))?;
    if session.revoked_at.is_some() || session.expires_at <= Utc::now() {
        let _ =
            nvbes_redis::session::delete_session(redis, &session.principal_id, &session.session_id)
                .await;
        return Err(AppError::unauthorized(
            "session_expired",
            "Session expired.",
        ));
    }
    Ok(session_view_from_cached_session(&session, current))
}

pub async fn revoke_all_user_sessions(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    revoke_all_user_sessions_tx(&mut tx, user_id).await?;
    tx.commit().await?;
    nvbes_redis::session::clear_user_sessions(redis, &user_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;
    Ok(())
}

pub async fn revoke_all_user_sessions_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), AppError> {
    let _ = tx;
    let _ = user_id;
    Ok(())
}

#[cfg(test)]
#[path = "identity.domains.auth.sessions.mgmt.tests.rs"]
mod tests;
