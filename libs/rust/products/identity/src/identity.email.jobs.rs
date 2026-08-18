use nvbes_email::EmailCommand;
use serde_json::json;

use crate::{IdentityError, IdentityResult};

pub const JOB_EMAIL_SUBMIT: &str = "integration.email.submit";
pub const JOB_DATA_EXPORT: &str = "account.data_export";

pub async fn enqueue_email_command(
    redis: &nvbes_redis::RedisPool,
    command: EmailCommand,
) -> IdentityResult<()> {
    let idempotency_key = command.idempotency_key.as_str().to_string();
    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_EMAIL_SUBMIT.to_string(),
            job_type: JOB_EMAIL_SUBMIT.to_string(),
            payload: json!(command),
            idempotency_key: Some(idempotency_key),
            max_attempts: 5,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .map_err(|error| {
        IdentityError::internal("redis_worker_queue_enqueue_failed", error.to_string())
    })?;
    Ok(())
}

pub async fn enqueue_data_export_job_tx(
    redis: &nvbes_redis::RedisPool,
    user_id: uuid::Uuid,
    user_email: &str,
) -> IdentityResult<()> {
    let payload = serde_json::json!({
        "user_id": user_id,
        "user_email": user_email,
    });

    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_DATA_EXPORT.to_string(),
            job_type: JOB_DATA_EXPORT.to_string(),
            payload,
            idempotency_key: Some(format!("data_export:{user_id}")),
            max_attempts: 2,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .map_err(|error| {
        IdentityError::internal("redis_worker_queue_enqueue_failed", error.to_string())
    })?;

    Ok(())
}
