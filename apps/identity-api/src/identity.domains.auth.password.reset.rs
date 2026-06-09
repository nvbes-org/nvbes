use chrono::Utc;
use nvbes_core::config::AppConfig;
use sqlx::PgPool;

use super::db;
use super::hash_password;
use super::history;
use super::token_hash;
use super::validate_password;
use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::domains::auth::types::{ResetPasswordInput, ResetPasswordResult};
use crate::http::error::AppError;

pub async fn reset(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: ResetPasswordInput,
) -> Result<ResetPasswordResult, AppError> {
    validate_password(&input.new_password)?;

    let token_hash = token_hash(&input.token);
    let (principal_id, expires_at, consumed_at) =
        db::reset::find_token_for_reset(redis, &token_hash)
            .await?
            .ok_or_else(|| AppError::not_found("reset_token_not_found", "Invalid reset token."))?;

    if consumed_at.is_some() || expires_at <= Utc::now() {
        return Err(AppError::forbidden(
            "reset_token_expired",
            "Reset token is expired or already used.",
        ));
    }

    if history::is_password_reused(
        db,
        principal_id,
        &input.new_password,
        config.auth_password_history_size,
    )
    .await?
    {
        return Err(AppError::bad_request(
            "password_reused",
            "This password has been used recently. Please choose a different one.",
        ));
    }

    let new_hash = hash_password(&input.new_password)?;
    let mut tx = db.begin().await?;
    db::reset::apply_password_reset(&mut tx, principal_id, &token_hash, &new_hash).await?;
    crate::domains::auth::sessions_mgmt::revoke_all_user_sessions_tx(&mut tx, principal_id).await?;
    tx.commit().await?;

    nvbes_redis::session::clear_user_sessions(redis, &principal_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))?;
    nvbes_redis::password_reset::mark_password_reset_token_consumed(redis, &token_hash)
        .await
        .map_err(|err| {
            AppError::internal("password_reset_token_consume_failed", err.to_string())
        })?;

    history::insert_password_hash(db, principal_id, &new_hash).await?;
    history::prune_history(db, principal_id, config.auth_password_history_size).await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "password_reset_completed".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 10.0,
            risk_factors: serde_json::json!({ "action": "password_reset" }),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({}),
        },
    )
    .await;

    nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, principal_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;

    Ok(ResetPasswordResult { success: true })
}
