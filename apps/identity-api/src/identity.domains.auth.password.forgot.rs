use chrono::{Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

use super::db;
use super::generate_random_token;
use super::log_dev_token;
use super::normalize_email;
use super::token_hash;
use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::domains::auth::types::{ForgotPasswordInput, ForgotPasswordResult};
use crate::http::error::AppError;

use super::geo_impl::{geo_metadata, password_geo_signal};

pub async fn forgot(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: ForgotPasswordInput,
    reset_ttl_minutes: i64,
    environment: &str,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ForgotPasswordResult, AppError> {
    let email = normalize_email(&input.email);
    let principal_data = db::find_principal_and_display_name_by_email(db, &email).await?;

    if let Some((principal_id, display_name)) = principal_data {
        let (tenant_id, tenant_kind) = db::get_tenant_info_by_principal(db, principal_id).await?;

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
                    ip_address: ip.clone(),
                    user_agent: user_agent.clone(),
                    risk_score,
                    risk_factors: risk_factors.clone(),
                    decision,
                    metadata: serde_json::json!({
                        "tenant_kind": tenant_kind,
                        "available_at": available_at,
                        "geo": geo_metadata(geo_resolution.as_ref()),
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
