use chrono::{Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use std::time::{Duration, Instant};

use super::db;
use super::generate_random_token;
use super::normalize_email;
use super::review::{self, RecoveryReviewDisposition};
use super::token_hash;
use crate::domains::auth::risk::{self, RiskEventInput};
use crate::domains::auth::types::{ForgotPasswordInput, ForgotPasswordResult};
use crate::http::error::AppError;

use super::geo_impl::{geo_metadata, password_geo_signal};

const MINIMUM_FORGOT_RESPONSE_TIME: Duration = Duration::from_millis(250);

pub async fn forgot(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: ForgotPasswordInput,
    reset_ttl_minutes: i64,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ForgotPasswordResult, AppError> {
    let started_at = Instant::now();
    let result = forgot_inner(db, redis, config, input, reset_ttl_minutes, ip, user_agent).await;
    if let Some(remaining) = MINIMUM_FORGOT_RESPONSE_TIME.checked_sub(started_at.elapsed()) {
        tokio::time::sleep(remaining).await;
    }
    if let Err(error) = result {
        tracing::error!(
            error.code = %error.code,
            "password reset request could not be completed"
        );
    }
    Ok(ForgotPasswordResult { success: true })
}

async fn forgot_inner(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: ForgotPasswordInput,
    reset_ttl_minutes: i64,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ForgotPasswordResult, AppError> {
    let email = normalize_email(&input.email);
    let principal_data = db::find_principal_and_display_name_by_email(db, &email).await?;

    if let Some((principal_id, display_name)) = principal_data {
        let (risk_score, decision, risk_factors) =
            risk::current_state_summary(db, principal_id).await?;
        let (risk_score, risk_factors, geo_decision, geo_resolution) = password_geo_signal(
            db,
            config,
            principal_id,
            ip.as_deref(),
            risk_score,
            risk_factors,
            "password_reset_requested",
        )
        .await;
        let decision = decision.strictest(geo_decision);
        let _ = risk::record_event(
            db,
            RiskEventInput {
                principal_id,
                session_id: None,
                device_id: None,
                event_type: "password_reset_requested".to_string(),
                ip_address: ip.clone(),
                user_agent: user_agent.clone(),
                risk_score,
                risk_factors: risk_factors.clone(),
                decision,
                metadata: serde_json::json!({
                    "email": email,
                    "geo": geo_metadata(geo_resolution.as_ref()),
                }),
            },
        )
        .await;

        let reset_locked = risk::should_lock_password_reset(db, principal_id).await?;
        let approved_review_id = if review::requires_manual_review(decision, reset_locked) {
            match review::queue_or_authorize(db, principal_id, &email, risk_score, &risk_factors)
                .await?
            {
                RecoveryReviewDisposition::Pending { request_id } => {
                    if let Some(request_id) = request_id
                        && let Err(error) =
                            crate::domains::auth::email_addresses::notify_recovery_review_requested(
                                db,
                                redis,
                                principal_id,
                                request_id,
                            )
                            .await
                    {
                        tracing::error!(
                            principal_id = %principal_id,
                            request_id = %request_id,
                            error = ?error,
                            "failed to enqueue recovery review notification"
                        );
                    }
                    return Ok(ForgotPasswordResult { success: true });
                }
                RecoveryReviewDisposition::Approved {
                    request_id,
                    tenant_id,
                } => Some((request_id, tenant_id)),
            }
        } else {
            None
        };

        let token = generate_random_token();
        let hashed_token = token_hash(&token);
        let expires_at = Utc::now() + ChronoDuration::minutes(reset_ttl_minutes);
        let mut tx = db.begin().await?;
        db::reset::insert_password_reset_token_tx(
            &mut tx,
            redis,
            principal_id,
            hashed_token.clone(),
            expires_at,
        )
        .await?;

        tx.commit().await?;

        if let Some((request_id, tenant_id)) = approved_review_id
            && let Err(error) =
                review::mark_token_issued(db, tenant_id, request_id, &hashed_token, expires_at)
                    .await
        {
            let _ =
                nvbes_redis::password_reset::take_password_reset_token(redis, &hashed_token).await;
            return Err(error);
        }
        let enqueue_result = crate::email::commands::enqueue(
            redis,
            email.clone(),
            Some(display_name.clone()),
            format!("reset:{hashed_token}"),
            nvbes_email::EmailTemplate::PasswordResetV1 {
                user_name: display_name,
                reset_url: crate::email::commands::password_reset_url(config, &token),
                credential_expires_at: expires_at,
            },
            expires_at,
            Some(principal_id),
        )
        .await;
        if let Err(error) = enqueue_result {
            let _ =
                nvbes_redis::password_reset::take_password_reset_token(redis, &hashed_token).await;
            if let Some((request_id, tenant_id)) = approved_review_id {
                let _ =
                    review::rollback_token_delivery(db, tenant_id, request_id, &hashed_token).await;
            }
            return Err(error.into());
        }

        Ok(ForgotPasswordResult { success: true })
    } else {
        Ok(ForgotPasswordResult { success: true })
    }
}
