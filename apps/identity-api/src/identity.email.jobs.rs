use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

use super::db::is_email_suppressed;
use super::webhooks::EmailProviderEvent;
use crate::http::error::AppError;

pub const JOB_EMAIL_SEND: &str = "email.send";
pub const JOB_EMAIL_WEBHOOK_PROCESS: &str = "email.webhook.process";
pub const JOB_DATA_EXPORT: &str = "data.export";

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailSendPayload {
    pub to_email: String,
    pub to_name: Option<String>,
    pub subject: String,
    pub html_body: String,
    pub text_body: Option<String>,
    pub business_type: String,
}

pub async fn enqueue_email_job_tx(
    pool: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    payload: EmailSendPayload,
    idempotency_key: &str,
) -> Result<(), AppError> {
    let essential_transactional_email = is_essential_transactional_email(&payload.business_type);

    if !essential_transactional_email && is_email_suppressed(pool, &payload.to_email).await? {
        warn!(
            email = %payload.to_email,
            subject = %payload.subject,
            "Skipping email send: recipient is suppressed"
        );
        return Ok(());
    }

    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_EMAIL_SEND.to_string(),
            job_type: JOB_EMAIL_SEND.to_string(),
            payload: json!(payload),
            idempotency_key: Some(idempotency_key.to_string()),
            max_attempts: 3,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("redis_worker_queue_enqueue_failed", err.to_string()))?;

    Ok(())
}

pub fn is_essential_transactional_email(business_type: &str) -> bool {
    matches!(business_type, "verification" | "password_reset")
}

pub async fn enqueue_email_event_job_tx(
    redis: &nvbes_redis::RedisPool,
    event: &EmailProviderEvent,
) -> Result<(), AppError> {
    let payload = serde_json::json!({
        "provider_event_id": event.id,
        "provider_email_id": event.email_id,
        "email": event.email,
        "event_type": event.event_type,
    });

    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_EMAIL_WEBHOOK_PROCESS.to_string(),
            job_type: JOB_EMAIL_WEBHOOK_PROCESS.to_string(),
            payload,
            idempotency_key: Some(format!("email:{}", event.id)),
            max_attempts: 2,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("redis_worker_queue_enqueue_failed", err.to_string()))?;

    Ok(())
}

pub async fn enqueue_data_export_job_tx(
    redis: &nvbes_redis::RedisPool,
    user_id: uuid::Uuid,
    email: &str,
) -> Result<(), AppError> {
    let payload = serde_json::json!({
        "user_id": user_id,
        "email": email,
    });

    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_DATA_EXPORT.to_string(),
            job_type: JOB_DATA_EXPORT.to_string(),
            payload,
            idempotency_key: Some(format!("export:{}", user_id)),
            max_attempts: 3,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .map_err(|err| AppError::internal("redis_worker_queue_enqueue_failed", err.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_essential_transactional_email;

    #[test]
    fn treats_security_messages_as_essential() {
        assert!(is_essential_transactional_email("verification"));
        assert!(is_essential_transactional_email("password_reset"));
    }

    #[test]
    fn treats_export_and_marketing_messages_as_suppressible() {
        assert!(!is_essential_transactional_email("data_export"));
        assert!(!is_essential_transactional_email("invitation"));
    }
}
