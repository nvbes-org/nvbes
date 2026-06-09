use crate::http::error::AppError;
use serde_json::Value;

pub const JOB_STRIPE_WEBHOOK_PROCESS: &str = "billing.stripe.webhook.process";
const DEFAULT_MAX_ATTEMPTS: u32 = 5;

pub async fn enqueue_stripe_webhook_job_tx(
    redis: &nvbes_redis::RedisPool,
    payload: Value,
    provider_event_id: &str,
) -> Result<(), AppError> {
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
    .map_err(|err| AppError::internal("redis_worker_queue_enqueue_failed", err.to_string()))?;

    Ok(())
}
