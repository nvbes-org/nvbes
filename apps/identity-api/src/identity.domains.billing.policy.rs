use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::http::error::AppError;
use nvbes_billing::models::BillingStateRecord;
use url::Url;
use uuid::Uuid;

pub fn resolve_billing_redirect_url(
    value: Option<&str>,
    default_url: &str,
    primary_origin: &str,
    staging_origin: Option<&str>,
    field_name: &'static str,
    env_name: &'static str,
) -> Result<String, AppError> {
    let candidate = value.unwrap_or(default_url).trim();
    let candidate_url = Url::parse(candidate).map_err(|_| {
        AppError::bad_request(
            "invalid_billing_return_url",
            &format!("{field_name} must be a valid absolute URL."),
        )
    })?;

    let on_primary_origin = url_matches_allowed_origin(&candidate_url, primary_origin);
    let on_staging_origin =
        staging_origin.is_some_and(|origin| url_matches_allowed_origin(&candidate_url, origin));

    if !on_primary_origin && !on_staging_origin {
        return Err(AppError::bad_request(
            "invalid_billing_return_url",
            &format!(
                "{field_name} must stay on the configured application origin. Set {env_name} to an allowed URL."
            ),
        ));
    }

    Ok(candidate_url.to_string())
}

pub fn url_matches_allowed_origin(candidate: &Url, allowed: &str) -> bool {
    let allowed = match Url::parse(allowed.trim()) {
        Ok(url) => url,
        Err(_) => return false,
    };

    candidate.scheme() == allowed.scheme()
        && candidate.host_str() == allowed.host_str()
        && candidate.port_or_known_default() == allowed.port_or_known_default()
}

pub fn subscription_status_requires_lock(status: &str) -> bool {
    matches!(status, "past_due" | "canceled" | "suspended" | "incomplete")
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

#[cfg(test)]
mod tests {
    use super::{resolve_billing_redirect_url, subscription_status_requires_lock};

    #[test]
    fn resolve_billing_redirect_url_accepts_allowed_origin() {
        let url = resolve_billing_redirect_url(
            Some("https://app.example.com/billing/success?workspace=1"),
            "https://app.example.com/billing/success",
            "https://app.example.com",
            Some("https://staging.example.com"),
            "success_url",
            "NVBES_BILLING_SUCCESS_URL",
        )
        .expect("expected allowed origin to be accepted");

        assert_eq!(url, "https://app.example.com/billing/success?workspace=1");
    }

    #[test]
    fn resolve_billing_redirect_url_rejects_external_origin() {
        let err = resolve_billing_redirect_url(
            Some("https://evil.example/phish"),
            "https://app.example.com/billing/success",
            "https://app.example.com",
            Some("https://staging.example.com"),
            "success_url",
            "NVBES_BILLING_SUCCESS_URL",
        )
        .expect_err("expected external origin to be rejected");

        assert_eq!(err.code, "invalid_billing_return_url");
    }

    #[test]
    fn subscription_status_requires_lock_blocks_degraded_states() {
        assert!(subscription_status_requires_lock("past_due"));
        assert!(subscription_status_requires_lock("canceled"));
        assert!(subscription_status_requires_lock("suspended"));
        assert!(subscription_status_requires_lock("incomplete"));
        assert!(!subscription_status_requires_lock("active"));
        assert!(!subscription_status_requires_lock("trialing"));
    }
}
