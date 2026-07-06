use anyhow::Context;
use nvbes_redis::worker_queue::QueuedJob;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::payloads::EmailSendPayload;
use crate::worker::BillingWorkerState;

pub(crate) async fn process_billing_email_job(
    state: &BillingWorkerState,
    job: &QueuedJob,
) -> anyhow::Result<Value> {
    let payload: EmailSendPayload =
        serde_json::from_value(job.payload.clone()).context("Invalid billing email job payload")?;
    let to_email = payload.to_email.clone();
    let subject = payload.subject.clone();
    let reply_to = state
        .config
        .email_reply_to
        .clone()
        .unwrap_or_else(|| to_email.clone());

    let message = nvbes_email::EmailMessage {
        from: email_from_address(&state.config)?,
        to: vec![nvbes_email::EmailAddress {
            email: to_email.clone(),
            name: payload.to_name,
        }],
        subject: subject.clone(),
        html_body: Some(payload.html_body),
        text_body: payload.text_body,
        headers: vec![
            ("Reply-To".to_string(), reply_to),
            ("X-Nvbes-Email-Job-Id".to_string(), job.id.to_string()),
            ("X-Nvbes-Email-Domain".to_string(), "billing".to_string()),
        ],
    };

    let result = state
        .email
        .send_message(&message)
        .await
        .context("Billing email delivery failed")?;

    let mut tx = state.db.begin().await?;
    record_email_sent_event_tx(&mut tx, &to_email, &result.provider_email_id, &subject).await?;
    record_email_message_tx(
        &mut tx,
        job.id,
        &payload.business_type,
        &to_email,
        &result.provider_email_id,
    )
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": "sent",
        "domain": "billing",
        "provider_email_id": result.provider_email_id,
    }))
}

fn email_from_address(
    config: &nvbes_core::config::AppConfig,
) -> anyhow::Result<nvbes_email::EmailAddress> {
    let email = match config.email_from_email.clone() {
        Some(email) => email,
        None if config.environment == "development" => "dev@nvbes.local".to_string(),
        None => anyhow::bail!("NVBES_EMAIL_FROM_EMAIL must be set for email sending"),
    };

    Ok(nvbes_email::EmailAddress {
        email,
        name: Some(
            config
                .email_from_name
                .clone()
                .unwrap_or_else(|| "nvbes".to_string()),
        ),
    })
}

async fn record_email_sent_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    provider_email_id: &str,
    subject: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO email_events (
          provider_event_id,
          provider_email_id,
          email,
          event_type,
          occurred_at,
          details
        )
        VALUES ($1, $2, $3, 'email_sent'::email_event_type, NOW(), $4)
        ON CONFLICT (provider_event_id) DO NOTHING
        "#,
    )
    .bind(format!("sent:{provider_email_id}"))
    .bind(provider_email_id)
    .bind(email)
    .bind(sqlx::types::Json(
        serde_json::json!({"subject": subject, "domain": "billing"}),
    ))
    .execute(tx.as_mut())
    .await
    .context("failed to record billing email sent event")?;

    Ok(())
}

async fn record_email_message_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: uuid::Uuid,
    business_type: &str,
    recipient_email: &str,
    provider_email_id: &str,
) -> anyhow::Result<()> {
    let recipient_hash = nvbes_billing::hex_encode(&Sha256::digest(recipient_email.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO email_messages (
            job_id,
            business_type,
            recipient_email,
            recipient_hash,
            provider_email_id,
            status
        )
        VALUES ($1, $2, $3, $4, $5, 'sent')
        "#,
    )
    .bind(job_id)
    .bind(business_type)
    .bind(recipient_email)
    .bind(recipient_hash)
    .bind(provider_email_id)
    .execute(tx.as_mut())
    .await
    .context("failed to record billing email message")?;

    Ok(())
}
