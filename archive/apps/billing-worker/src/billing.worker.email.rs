use anyhow::Context;
use chrono::{Duration, Utc};
use nvbes_billing::{StripeWebhookEvent, provider::ProviderPayment};
use nvbes_email::{
    EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext,
    EmailTemplate,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "billing.worker.email.delivery.rs"]
mod delivery;
#[path = "billing.worker.email.payloads.rs"]
mod payloads;

pub(crate) use delivery::process_billing_email_job;

pub(crate) async fn enqueue_billing_email_for_stripe_event(
    db_pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
    event: &StripeWebhookEvent,
) -> anyhow::Result<()> {
    let Some(template) = payloads::stripe_event_template(event) else {
        return Ok(());
    };
    let idempotency_key = payloads::billing_email_idempotency_key(&event.id, &template);
    enqueue_command(db_pool, redis, workspace_id, template, idempotency_key).await
}

pub(crate) async fn enqueue_billing_email_for_provider_payment(
    db_pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
    provider_event_id: &str,
    payment: &ProviderPayment,
) -> anyhow::Result<()> {
    let Some(template) = payloads::provider_payment_template(payment) else {
        return Ok(());
    };
    let idempotency_key = payloads::billing_email_idempotency_key(provider_event_id, &template);
    enqueue_command(db_pool, redis, workspace_id, template, idempotency_key).await
}

async fn enqueue_command(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
    template: EmailTemplate,
    idempotency_key: String,
) -> anyhow::Result<()> {
    let (email, name) = billing_email_recipient(pool, workspace_id).await?;
    let request_id = Uuid::new_v4().to_string();
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: request_id.clone(),
            correlation_id: request_id,
            actor_principal_id: String::new(),
        },
        producer: "billing-worker".to_string(),
        idempotency_key: EmailIdempotencyKey::new(idempotency_key)?,
        recipient: EmailRecipient { email, name },
        category: EmailCategory::Billing,
        template,
        deliver_before: Utc::now() + Duration::hours(24),
    };
    command.validate(Utc::now())?;
    let queue = nvbes_billing::jobs::JOB_BILLING_EMAIL_SUBMIT;
    let key = command.idempotency_key.as_str().to_string();
    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: queue.to_string(),
            job_type: queue.to_string(),
            payload: json!(command),
            idempotency_key: Some(key),
            max_attempts: 5,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .context("failed to enqueue billing email command")?;
    Ok(())
}

async fn billing_email_recipient(
    pool: &PgPool,
    workspace_id: Uuid,
) -> anyhow::Result<(String, Option<String>)> {
    let mut tx = pool.begin().await?;
    let billing = nvbes_billing::db::fetch_billing_state_tx(&mut tx, workspace_id)
        .await?
        .with_context(|| format!("workspace {workspace_id} not found for billing email"))?;
    tx.commit().await?;
    Ok((
        billing
            .billing_email
            .clone()
            .unwrap_or_else(|| billing.owner_email.clone()),
        Some(billing.owner_email),
    ))
}
