use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::{bot_response, risk};
use crate::http::error::AppError;

pub async fn record_failed_login(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Option<Uuid>,
    normalized_email: &str,
    ip: Option<String>,
    user_agent: Option<String>,
    reason: &'static str,
) -> Result<(), AppError> {
    let assessment = nvbes_redis::credential_stuffing::record_failed_login(
        redis,
        ip.as_deref(),
        normalized_email,
    )
    .await
    .map_err(|err| AppError::internal("credential_stuffing_state_failed", err.to_string()))?;

    if assessment.decision != nvbes_redis::credential_stuffing::CredentialStuffingDecision::Allow {
        record_risk_event(
            db,
            principal_id,
            &assessment,
            ip.clone(),
            user_agent.clone(),
            reason,
        )
        .await;
    }

    if assessment.decision == nvbes_redis::credential_stuffing::CredentialStuffingDecision::Block {
        bot_response::record_decision("block", "credential_stuffing");
        bot_response::apply_tarpit(1.0).await;
        return Err(blocked_error());
    }

    Ok(())
}

pub async fn successful_password_risk_score(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    normalized_email: &str,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<f64, AppError> {
    let assessment =
        nvbes_redis::credential_stuffing::assess_current(redis, ip.as_deref(), normalized_email)
            .await
            .map_err(|err| {
                AppError::internal("credential_stuffing_state_failed", err.to_string())
            })?;

    match assessment.decision {
        nvbes_redis::credential_stuffing::CredentialStuffingDecision::Allow => Ok(0.0),
        nvbes_redis::credential_stuffing::CredentialStuffingDecision::StepUp => {
            record_risk_event(
                db,
                Some(principal_id),
                &assessment,
                ip,
                user_agent,
                "valid_password_after_stuffing_signals",
            )
            .await;
            Ok(assessment.score as f64)
        }
        nvbes_redis::credential_stuffing::CredentialStuffingDecision::Block => {
            record_risk_event(
                db,
                Some(principal_id),
                &assessment,
                ip,
                user_agent,
                "valid_password_blocked_by_stuffing_signals",
            )
            .await;
            bot_response::record_decision("block", "credential_stuffing_valid_password");
            bot_response::apply_tarpit(1.0).await;
            Err(blocked_error())
        }
    }
}

async fn record_risk_event(
    db: &PgPool,
    principal_id: Option<Uuid>,
    assessment: &nvbes_redis::credential_stuffing::CredentialStuffingAssessment,
    ip: Option<String>,
    user_agent: Option<String>,
    reason: &'static str,
) {
    let Some(principal_id) = principal_id else {
        tracing::warn!(
            score = assessment.score,
            reasons = ?assessment.reasons,
            reason,
            "Credential stuffing suspected for unresolved account"
        );
        return;
    };

    let decision = match assessment.decision {
        nvbes_redis::credential_stuffing::CredentialStuffingDecision::Allow => {
            risk::RiskDecision::Allow
        }
        nvbes_redis::credential_stuffing::CredentialStuffingDecision::StepUp => {
            risk::RiskDecision::StepUp
        }
        nvbes_redis::credential_stuffing::CredentialStuffingDecision::Block => {
            risk::RiskDecision::Deny
        }
    };

    let _ = risk::record_event(
        db,
        risk::RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "credential_stuffing_suspected".to_string(),
            ip_address: ip,
            user_agent,
            risk_score: assessment.score as f64,
            risk_factors: serde_json::json!({
                "reasons": assessment.reasons,
                "ip_failed_accounts_short": assessment.stats.ip_failed_accounts_short,
                "ip_failed_accounts_long": assessment.stats.ip_failed_accounts_long,
                "account_failed_sources_short": assessment.stats.account_failed_sources_short,
                "account_failed_sources_long": assessment.stats.account_failed_sources_long,
            }),
            decision,
            metadata: serde_json::json!({
                "reason": reason,
                "control": "credential_stuffing_prevention",
            }),
        },
    )
    .await;
}

fn blocked_error() -> AppError {
    AppError::forbidden(
        "credential_stuffing_blocked",
        "Automated credential attack detected. Try again later.",
    )
}
