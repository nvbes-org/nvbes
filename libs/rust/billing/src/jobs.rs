use serde_json::Value;
use thiserror::Error;

pub const JOB_STRIPE_WEBHOOK_PROCESS: &str = "billing.stripe.webhook.process";
pub const JOB_MOLLIE_WEBHOOK_PROCESS: &str = "billing.mollie.webhook.process";
pub const JOB_BILLING_EMAIL_SEND: &str = "billing.email.send";
const DEFAULT_MAX_ATTEMPTS: u32 = 5;

#[derive(Debug, Error)]
pub enum BillingJobQueueError {
    #[error("billing job enqueue failed: {0}")]
    Enqueue(String),
}

pub async fn enqueue_stripe_webhook_job(
    redis: &nvbes_redis::RedisPool,
    payload: Value,
    provider_event_id: &str,
) -> Result<(), BillingJobQueueError> {
    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_STRIPE_WEBHOOK_PROCESS.to_string(),
            job_type: JOB_STRIPE_WEBHOOK_PROCESS.to_string(),
            payload,
            idempotency_key: Some(provider_event_id.to_string()),
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            overwrite_terminal: true,
            job_id: None,
        },
    )
    .await
    .map_err(|err| BillingJobQueueError::Enqueue(err.to_string()))?;

    Ok(())
}

pub async fn enqueue_mollie_webhook_job(
    redis: &nvbes_redis::RedisPool,
    provider_event_id: &str,
) -> Result<(), BillingJobQueueError> {
    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_MOLLIE_WEBHOOK_PROCESS.to_string(),
            job_type: JOB_MOLLIE_WEBHOOK_PROCESS.to_string(),
            payload: serde_json::json!({
                "provider_event_id": provider_event_id,
            }),
            idempotency_key: Some(provider_event_id.to_string()),
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            overwrite_terminal: true,
            job_id: None,
        },
    )
    .await
    .map_err(|err| BillingJobQueueError::Enqueue(err.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn billing_runtime_queues_are_provider_neutral_and_domain_scoped() {
        assert_eq!(
            super::JOB_STRIPE_WEBHOOK_PROCESS,
            "billing.stripe.webhook.process"
        );
        assert_eq!(
            super::JOB_MOLLIE_WEBHOOK_PROCESS,
            "billing.mollie.webhook.process"
        );
        assert_eq!(super::JOB_BILLING_EMAIL_SEND, "billing.email.send");
    }
}
