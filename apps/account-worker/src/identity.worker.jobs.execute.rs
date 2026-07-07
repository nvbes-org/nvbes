use anyhow::Context;
use nvbes_product_account::email::{
    db::{record_email_message_tx, record_email_sent_event_tx},
    jobs::{EmailSendPayload, JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS},
};
use serde_json::Value;

use crate::app::AppState;

use nvbes_redis::worker_queue::QueuedJob;

use super::{
    email_from_address, process_data_export::process_data_export,
    process_email_event::process_email_event,
};

pub(super) async fn execute_job(state: &AppState, job: &QueuedJob) -> anyhow::Result<Value> {
    match job.job_type.as_str() {
        JOB_EMAIL_SEND => send_email_job(state, job).await,
        JOB_EMAIL_WEBHOOK_PROCESS => {
            process_email_event(&state.db, &job.payload)
                .await
                .map_err(|e| anyhow::anyhow!(format!("Email event processing failed: {e:?}")))?;
            Ok(serde_json::json!({"status": "processed"}))
        }
        JOB_DATA_EXPORT => {
            process_data_export(state, &job.payload)
                .await
                .map_err(|e| anyhow::anyhow!("Data export failed: {e:?}"))?;
            Ok(serde_json::json!({"status": "exported"}))
        }
        _ => Err(anyhow::anyhow!("Unknown job type: {}", job.job_type)),
    }
}

async fn send_email_job(state: &AppState, job: &QueuedJob) -> anyhow::Result<Value> {
    let payload: EmailSendPayload = serde_json::from_value(job.payload.clone())?;
    let to_email = payload.to_email.clone();
    let subject = payload.subject.clone();
    let reply_to = state
        .config
        .email_reply_to
        .clone()
        .unwrap_or_else(|| to_email.clone());

    let msg = nvbes_email::EmailMessage {
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
        ],
    };

    let result = state
        .email
        .send_message(&msg)
        .await
        .context("Email delivery failed")?;

    let mut tx = state.db.begin().await?;
    let business_type = &payload.business_type;
    record_email_sent_event_tx(&mut tx, &to_email, &result.provider_email_id, &subject)
        .await
        .map_err(|e| anyhow::anyhow!(format!("Failed to record sent event: {e:?}")))?;
    record_email_message_tx(
        &mut tx,
        job.id,
        business_type,
        &to_email,
        &result.provider_email_id,
    )
    .await
    .map_err(|e| anyhow::anyhow!(format!("Failed to record email message: {e:?}")))?;
    tx.commit().await?;

    Ok(serde_json::json!({"status": "sent", "provider_email_id": result.provider_email_id}))
}
