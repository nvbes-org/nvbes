use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::http::error::AppError;
use nvbes_billing::{
    BillingRedirectUrlError, models::BillingStateRecord, subscription_status_requires_lock,
};
use uuid::Uuid;

pub fn resolve_billing_redirect_url(
    value: Option<&str>,
    default_url: &str,
    primary_origin: &str,
    staging_origin: Option<&str>,
    field_name: &'static str,
    env_name: &'static str,
) -> Result<String, AppError> {
    nvbes_billing::resolve_billing_redirect_url(value, default_url, primary_origin, staging_origin)
        .map_err(|error| match error {
            BillingRedirectUrlError::InvalidAbsoluteUrl => AppError::bad_request(
                "invalid_billing_return_url",
                &format!("{field_name} must be a valid absolute URL."),
            ),
            BillingRedirectUrlError::InvalidOrigin => AppError::bad_request(
                "invalid_billing_return_url",
                &format!(
                    "{field_name} must stay on the configured application origin. Set {env_name} to an allowed URL."
                ),
            ),
        })
}

pub async fn enforce_billing_rate_limits(
    limiter: &nvbes_core::limiter::RateLimiter,
    workspace_id: Uuid,
    user_id: Uuid,
    action: &str,
) -> Result<(), AppError> {
    limiter
        .check(
            &format!("billing:{action}:workspace"),
            &workspace_id.to_string(),
            4,
            std::time::Duration::from_secs(900),
        )
        .await?;
    limiter
        .check(
            &format!("billing:{action}:user"),
            &user_id.to_string(),
            6,
            std::time::Duration::from_secs(900),
        )
        .await?;
    Ok(())
}

pub async fn enforce_billing_risk_policy(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
    billing_record: &BillingStateRecord,
) -> Result<(), AppError> {
    let (score, decision, factors) = risk::current_state_summary(db, user_id).await?;
    let subscription_locked =
        subscription_status_requires_lock(&billing_record.subscription_status);

    if subscription_locked
        || matches!(decision, RiskDecision::Deny | RiskDecision::Lock)
        || score >= 50.0
    {
        let _ = risk::record_event(
            db,
            RiskEventInput {
                principal_id: user_id,
                session_id: None,
                device_id: None,
                event_type: "billing_operation_blocked".to_string(),
                ip_address: None,
                user_agent: None,
                risk_score: score,
                risk_factors: factors,
                decision: RiskDecision::Deny,
                metadata: serde_json::json!({
                    "workspace_id": workspace_id,
                    "subscription_status": billing_record.subscription_status,
                }),
            },
        )
        .await;

        return Err(AppError::forbidden(
            "billing_locked",
            "Billing actions are temporarily blocked for this workspace.",
        ));
    }

    Ok(())
}
