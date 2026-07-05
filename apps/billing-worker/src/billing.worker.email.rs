use anyhow::Context;
use nvbes_billing::{StripeWebhookEvent, provider::ProviderPayment};
use serde_json::json;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

#[path = "billing.worker.email.delivery.rs"]
mod delivery;
#[path = "billing.worker.email.payloads.rs"]
mod payloads;

const EMAIL_MAX_ATTEMPTS: u32 = 3;
pub(crate) use delivery::process_billing_email_job;

pub(crate) async fn enqueue_billing_email_for_stripe_event(
    db_pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
    event: &StripeWebhookEvent,
) -> anyhow::Result<()> {
    let (to_email, to_name) = billing_email_recipient(db_pool, workspace_id).await?;
    let Some(payload) = payloads::stripe_event_payload(&to_email, to_name, event) else {
        return Ok(());
    };

    let idempotency_key = payloads::billing_email_idempotency_key(&event.id, &payload.business_type);
    enqueue_email_job(db_pool, redis, payload, &idempotency_key).await
}

pub(crate) async fn enqueue_billing_email_for_provider_payment(
    db_pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
    provider_event_id: &str,
    payment: &ProviderPayment,
) -> anyhow::Result<()> {
    let (to_email, to_name) = billing_email_recipient(db_pool, workspace_id).await?;
    let Some(payload) = payloads::provider_payment_payload(&to_email, to_name, payment) else {
        return Ok(());
    };

    let idempotency_key =
        payloads::billing_email_idempotency_key(provider_event_id, &payload.business_type);
    enqueue_email_job(db_pool, redis, payload, &idempotency_key).await
}

async fn billing_email_recipient(
    db_pool: &PgPool,
    workspace_id: Uuid,
) -> anyhow::Result<(String, Option<String>)> {
    let mut tx = db_pool
        .begin()
        .await
        .context("failed to start billing email lookup transaction")?;
    let billing = nvbes_billing::db::fetch_billing_state_tx(&mut tx, workspace_id)
        .await
        .context("failed to fetch billing state for billing email")?
        .with_context(|| format!("workspace {workspace_id} not found for billing email"))?;
    tx.commit()
        .await
        .context("failed to commit billing email lookup transaction")?;

    Ok((
        billing
            .billing_email
            .clone()
            .unwrap_or_else(|| billing.owner_email.clone()),
        Some(billing.owner_email),
    ))
}

async fn enqueue_email_job(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    payload: payloads::EmailSendPayload,
    idempotency_key: &str,
) -> anyhow::Result<()> {
    if !is_essential_transactional_email(&payload.business_type)
        && is_email_suppressed(pool, &payload.to_email).await?
    {
        warn!(
            email = %payload.to_email,
            subject = %payload.subject,
            "Skipping billing email send: recipient is suppressed"
        );
        return Ok(());
    }

    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND.to_string(),
            job_type: nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND.to_string(),
            payload: json!(payload),
            idempotency_key: Some(idempotency_key.to_string()),
            max_attempts: EMAIL_MAX_ATTEMPTS,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .context("failed to enqueue billing email job")?;

    Ok(())
}

async fn is_email_suppressed(db: &PgPool, email: &str) -> anyhow::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM suppressed_emails WHERE email = $1
        )
        "#,
    )
    .bind(email)
    .fetch_one(db)
    .await
    .context("failed to check suppressed email")
}

fn is_essential_transactional_email(business_type: &str) -> bool {
    matches!(
        business_type,
        "verification" | "password_reset" | "account_security"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn billing_email_uses_billing_worker_queue() {
        assert_eq!(
            nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND,
            "billing.email.send"
        );
    }
}
