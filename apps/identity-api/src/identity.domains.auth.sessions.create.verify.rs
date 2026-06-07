use chrono::Utc;
use nvbes_core::config::AppConfig;
use nvbes_core::limiter::RateLimiter;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::{password, risk, types::LoginInput};
use crate::http::error::AppError;

pub struct VerifiedPrimaryLogin {
    pub principal_id: Uuid,
    pub email: String,
}

pub async fn verify_primary_credentials(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: &LoginInput,
) -> Result<VerifiedPrimaryLogin, AppError> {
    let email = password::normalize_email(&input.email);
    RateLimiter::new(redis.clone())
        .check("login", &email, 12, std::time::Duration::from_secs(300))
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

    if let Some(max_age_days) = config.auth_password_max_age_days {
        let last_changed: Option<chrono::DateTime<chrono::Utc>> =
            row.get("password_last_changed_at");
        let expired = last_changed.map_or(true, |lc| {
            lc + chrono::Duration::days(max_age_days) <= Utc::now()
        });
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
        return Err(err);
    }

    Ok(VerifiedPrimaryLogin {
        principal_id,
        email,
    })
}
