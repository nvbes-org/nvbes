use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

use super::hash_password;
use super::history;
use super::{validate_password, verify_password};
use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::domains::auth::types::{ChangePasswordInput, ChangePasswordResult};
use crate::http::error::AppError;

pub async fn change(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    user_id: Uuid,
    current_session_id: Uuid,
    input: ChangePasswordInput,
) -> Result<ChangePasswordResult, AppError> {
    validate_password(&input.new_password)?;

    let user = crate::domains::auth::db::fetch_user_record(db, user_id).await?;
    let stored_hash = user.password_hash.as_deref().ok_or_else(|| {
        AppError::forbidden(
            "password_missing",
            "No password is configured for this account.",
        )
    })?;

    if input.current_password == input.new_password {
        return Err(AppError::bad_request(
            "validation_failed",
            "New password must be different from current password.",
        ));
    }

    verify_password(stored_hash, &input.current_password)?;

    if history::is_password_reused(
        db,
        user_id,
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
    sqlx::query("UPDATE users SET password_hash = $2, password_last_changed_at = NOW(), updated_at = NOW() WHERE principal_id = $1")
        .bind(user_id)
        .bind(&new_hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    history::insert_password_hash(db, user_id, stored_hash).await?;
    history::insert_password_hash(db, user_id, &new_hash).await?;
    history::prune_history(db, user_id, config.auth_password_history_size).await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: user_id,
            session_id: Some(current_session_id),
            device_id: None,
            event_type: "password_changed".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 0.0,
            risk_factors: serde_json::json!({}),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({}),
        },
    )
    .await;

    nvbes_redis::session::clear_user_sessions_except(
        redis,
        &user_id.to_string(),
        &current_session_id.to_string(),
    )
    .await
    .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))?;
    nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, user_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;

    Ok(ChangePasswordResult { success: true })
}
