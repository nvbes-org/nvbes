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
        JOB_STRIPE_WEBHOOK_PROCESS,
        JOB_STRIPE_WEBHOOK_PROCESS,
        payload,
        Some(provider_event_id),
        DEFAULT_MAX_ATTEMPTS,
        true,
        None,
    )
    .await
    .map_err(|err| AppError::internal("redis_worker_queue_enqueue_failed", &err.to_string()))?;

    Ok(())
}
