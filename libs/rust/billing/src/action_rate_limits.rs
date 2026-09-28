use std::time::Duration;

use nvbes_core::http::error::AppError;
use nvbes_core::limiter::RateLimiter;
use uuid::Uuid;

pub async fn enforce_billing_action_rate_limits(
    limiter: &RateLimiter,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    action: &str,
) -> Result<(), AppError> {
    limiter
        .check(
            &format!("billing:{action}:workspace"),
            &workspace_id.to_string(),
            4,
            Duration::from_secs(900),
        )
        .await?;
    limiter
        .check(
            &format!("billing:{action}:user"),
            &actor_principal_id.to_string(),
            6,
            Duration::from_secs(900),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "action_rate_limits.tests.rs"]
mod tests;
