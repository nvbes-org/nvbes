use chrono::Utc;
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::{
    audit::{AuthAuditInput, record_auth_event},
    login_throttle::{LOGIN_THROTTLE_ACTION, LoginThrottleKeys},
    password, risk,
    types::LoginInput,
};
use crate::http::error::AppError;

pub struct VerifiedPrimaryLogin {
    pub principal_id: Uuid,
    pub email: String,
    pub risk_score: f64,
}

pub async fn verify_primary_credentials(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: &LoginInput,
) -> Result<VerifiedPrimaryLogin, AppError> {
    let email = password::normalize_email(&input.email);
    let throttle_keys = LoginThrottleKeys::from_parts(input.ip.as_deref(), &email, None);
    let [ip_rule, account_rule] = throttle_keys.pre_lookup_rules();
    nvbes_core::limiter::check_rate_limit_pair(redis, LOGIN_THROTTLE_ACTION, ip_rule, account_rule)
        .await?;

    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          u.email,
          u.firstname,
          u.lastname,
          u.username,
          u.birthdate,
          u.region,
          u.password_hash,
          u.email_verified_at,
          u.password_last_changed_at,
          u.created_at,
          p.tenant_id
        FROM users u
        INNER JOIN principals p ON p.id = u.principal_id
        WHERE lower(u.email) = lower($1)
        LIMIT 1
        "#,
    )
    .bind(&email)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid_credentials", "Invalid email or password."))?;

    let principal_id: Uuid = row.get("principal_id");
    let tenant_id: Option<Uuid> = row.get("tenant_id");
    let throttle_keys = LoginThrottleKeys::from_parts(input.ip.as_deref(), &email, tenant_id);
    if let Some(tenant_rule) = throttle_keys.tenant_rule() {
        nvbes_core::limiter::check_rate_limit(
            redis,
            LOGIN_THROTTLE_ACTION,
            tenant_rule.key,
            tenant_rule.max_hits,
            tenant_rule.window,
        )
        .await?;
    }

    if let Some(max_age_days) = config.auth_password_max_age_days {
        let last_changed: Option<chrono::DateTime<chrono::Utc>> =
            row.get("password_last_changed_at");
        let expired =
            last_changed.is_none_or(|lc| lc + chrono::Duration::days(max_age_days) <= Utc::now());
        if expired {
            return Err(AppError::forbidden(
                "password_expired",
                "Your password has expired and must be changed.",
            ));
        }
    }

    let (risk_score, risk_decision, risk_factors) =
        risk::current_state_summary(db, principal_id).await?;
    if matches!(
        risk_decision,
        risk::RiskDecision::Deny | risk::RiskDecision::Lock
    ) {
        let _ = risk::record_event(
            db,
            risk::RiskEventInput {
                principal_id,
                session_id: None,
                device_id: None,
                event_type: "login_blocked".to_string(),
                ip_address: input.ip.clone(),
                user_agent: input.user_agent.clone(),
                risk_score,
                risk_factors: risk_factors.clone(),
                decision: risk_decision,
                metadata: serde_json::json!({
                    "email": email,
                    "reason": "suspicious_activity",
                }),
            },
        )
        .await;

        return Err(AppError::forbidden(
            "risk_policy_blocked",
            "This account is temporarily blocked due to suspicious activity.",
        ));
    }

    let password_hash: Option<String> = row.get("password_hash");
    let stored_hash = password_hash.ok_or_else(|| {
        AppError::unauthorized("invalid_credentials", "Invalid email or password.")
    })?;

    crate::domains::auth::login_delay::apply_login_delay(db, principal_id).await?;

    if let Err(err) = password::verify_password(&stored_hash, &input.password) {
        let _ = risk::record_event(
            db,
            risk::RiskEventInput {
                principal_id,
                session_id: None,
                device_id: None,
                event_type: "login_failed".to_string(),
                ip_address: input.ip.clone(),
                user_agent: input.user_agent.clone(),
                risk_score: 25.0,
                risk_factors: serde_json::json!({ "reason": "invalid_password" }),
                decision: risk::RiskDecision::StepUp,
                metadata: serde_json::json!({ "email": email }),
            },
        )
        .await;
        let _ = record_auth_event(
            db,
            AuthAuditInput {
                principal_id,
                action: "auth.login_failed",
                target_type: "principal",
                target_id: Some(principal_id),
                ip: input.ip.as_deref(),
                user_agent: input.user_agent.as_deref(),
                metadata: serde_json::json!({
                    "email": email,
                    "reason": "invalid_password",
                }),
            },
        )
        .await;
        return Err(err);
    }

    Ok(VerifiedPrimaryLogin {
        principal_id,
        email,
        risk_score,
    })
}
