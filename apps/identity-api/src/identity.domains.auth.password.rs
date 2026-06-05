use chrono::{Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

pub use core::{generate_token, normalize_email, unique_slug, validate_email, validate_password};
use nvbes_core::auth::helpers as core;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    Ok(core::hash_password(password)?)
}

pub fn verify_password(hash: &str, password: &str) -> Result<(), AppError> {
    if core::verify_password(password, hash)? {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "invalid_credentials",
            "Invalid email or password.",
        ))
    }
}

pub fn generate_random_token() -> String {
    core::generate_random_token()
}

pub fn token_hash(token: &str) -> String {
    core::token_hash(token)
}

pub fn log_dev_token(token: &str, environment: &str, purpose: &str) {
    core::log_dev_token(token, environment, purpose)
}

use super::risk::{self, RiskDecision, RiskEventInput};
use super::types::*;
use crate::http::error::AppError;

#[path = "identity.domains.auth.password.db.rs"]
pub mod db;
#[path = "identity.domains.auth.password.enterprise.rs"]
pub mod enterprise;
#[path = "identity.domains.auth.password.history.rs"]
pub mod history;

pub use enterprise::approve_enterprise_recovery;

pub async fn forgot(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: ForgotPasswordInput,
    reset_ttl_minutes: i64,
    environment: &str,
) -> Result<ForgotPasswordResult, AppError> {
    let email = normalize_email(&input.email);
    let principal_data = db::find_principal_and_display_name_by_email(db, &email).await?;

    if let Some((principal_id, display_name)) = principal_data {
        let (tenant_id, tenant_kind) = db::get_tenant_info_by_principal(db, principal_id).await?;

        let (risk_score, decision, risk_factors) =
            risk::current_state_summary(db, principal_id).await?;
        let _ = risk::record_event(
            db,
            RiskEventInput {
                principal_id,
                session_id: None,
                device_id: None,
                event_type: "password_reset_requested".to_string(),
                ip_address: None,
                user_agent: None,
                risk_score,
                risk_factors: risk_factors.clone(),
                decision,
                metadata: serde_json::json!({
                    "email": email,
                }),
            },
        )
        .await;

        if matches!(decision, RiskDecision::Deny | RiskDecision::Lock)
            || risk::should_lock_password_reset(db, principal_id).await?
        {
            return Err(AppError::forbidden(
                "risk_policy_blocked",
                "This account is temporarily blocked from password reset due to suspicious activity.",
            ));
        }

        if tenant_kind == "enterprise" {
            let available_at = Utc::now() + ChronoDuration::hours(24);
            let request_id = Uuid::new_v4();

            db::insert_enterprise_recovery_request(
                db,
                request_id,
                principal_id,
                tenant_id,
                &email,
                available_at,
            )
            .await?;

            let _ = risk::record_event(
                db,
                RiskEventInput {
                    principal_id,
                    session_id: None,
                    device_id: None,
                    event_type: "enterprise_recovery_requested".to_string(),
                    ip_address: None,
                    user_agent: None,
                    risk_score,
                    risk_factors: risk_factors.clone(),
                    decision,
                    metadata: serde_json::json!({
                        "tenant_kind": tenant_kind,
                        "available_at": available_at,
                    }),
                },
            )
            .await;

            return Ok(ForgotPasswordResult {
                success: true,
                requires_admin_approval: true,
                available_at: Some(available_at),
            });
        }

        let token = generate_random_token();
        log_dev_token(&token, environment, "password_reset");
        let mut tx = db.begin().await?;
        db::reset::insert_password_reset_token_tx(
            &mut tx,
            redis,
            principal_id,
            token_hash(&token),
            Utc::now() + ChronoDuration::minutes(reset_ttl_minutes),
        )
        .await?;

        let email_msg =
            crate::email::templates::password_reset_email(config, &email, &email, &token)?;

        tx.commit().await?;

        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            crate::email::jobs::EmailSendPayload {
                to_email: email.clone(),
                to_name: Some(display_name),
                subject: email_msg.subject,
                html_body: email_msg.html_body.unwrap_or_default(),
                text_body: email_msg.text_body,
                business_type: "password_reset".to_string(),
            },
            &format!("reset:{}", token_hash(&token)),
        )
        .await?;

        Ok(ForgotPasswordResult {
            success: true,
            requires_admin_approval: false,
            available_at: None,
        })
    } else {
        Ok(ForgotPasswordResult {
            success: true,
            requires_admin_approval: false,
            available_at: None,
        })
    }
}

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
        .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;
    nvbes_redis::password_reset::mark_password_reset_token_consumed(redis, &token_hash)
        .await
        .map_err(|err| {
            AppError::internal("password_reset_token_consume_failed", &err.to_string())
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
            risk_factors: serde_json::json!({
                "action": "password_reset",
            }),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({}),
        },
    )
    .await;

    nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, principal_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;

    Ok(ResetPasswordResult { success: true })
}

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
    .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;
    nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, user_id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;

    Ok(ChangePasswordResult { success: true })
}

#[cfg(test)]
#[path = "identity.domains.auth.password.tests.rs"]
mod tests;
