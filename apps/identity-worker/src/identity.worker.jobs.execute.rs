use anyhow::Context;
use serde_json::Value;

use crate::app::AppState;
use crate::domains::billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS;
use crate::domains::billing::webhooks::process_stripe_event;
use crate::email::jobs::{
    EmailSendPayload, JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
};
use crate::worker::analytics::capture_billing_webhook_analytics;

use nvbes_redis::worker_queue::QueuedJob;

use super::{process_data_export::process_data_export, process_email_event::process_email_event};

pub(super) async fn execute_job(state: &AppState, job: &QueuedJob) -> anyhow::Result<Value> {
    match job.job_type.as_str() {
        JOB_EMAIL_SEND => send_email_job(state, job).await,
        JOB_EMAIL_WEBHOOK_PROCESS => {
            process_email_event(&state.db, &job.payload)
                .await
                .map_err(|e| anyhow::anyhow!(format!("Email event processing failed: {e:?}")))?;
            Ok(serde_json::json!({"status": "processed"}))
        }
        JOB_STRIPE_WEBHOOK_PROCESS => process_stripe_webhook_job(state, job).await,
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

    let from_email = state
        .config
        .email_from_email
        .clone()
        .context("NVBES_EMAIL_FROM_EMAIL must be set for email sending")?;

    let msg = nvbes_email::EmailMessage {
        from: nvbes_email::EmailAddress {
            email: from_email,
            name: Some(
                state
                    .config
                    .email_from_name
                    .clone()
                    .unwrap_or_else(|| "nvbes".to_string()),
            ),
        },
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
    crate::email::db::record_email_sent_event_tx(
        &mut tx,
        &to_email,
        &result.provider_email_id,
        &subject,
    )
    .await
    .map_err(|e| anyhow::anyhow!(format!("Failed to record sent event: {e:?}")))?;
    crate::email::db::record_email_message_tx(
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

async fn process_stripe_webhook_job(state: &AppState, job: &QueuedJob) -> anyhow::Result<Value> {
    let job_payload: nvbes_billing::StripeWebhookEvent =
        serde_json::from_value(job.payload.clone())
            .context("Invalid Stripe webhook job payload")?;
    let mut tx = state.db.begin().await?;
    let workspace_id = process_stripe_event(&mut tx, &job_payload)
        .await
        .map_err(|e| anyhow::anyhow!(format!("Stripe webhook processing failed: {e:?}")))?;
    tx.commit().await?;

    if let Some(workspace_id) = workspace_id {
        let _ =
            nvbes_redis::pubsub::publish_workspace_updated(&state.redis, &workspace_id.to_string())
                .await;

        let plan_code: Option<String> = sqlx::query_scalar(
            "SELECT p.code FROM workspaces w JOIN plans p ON p.id = w.plan_id WHERE w.id = $1",
        )
        .bind(workspace_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        if let Some(code) = plan_code.as_deref() {
            let _ = nvbes_redis::pubsub::publish_workspace_plan_updated(
                &state.redis,
                &workspace_id.to_string(),
                code,
            )
            .await;
        }

        capture_billing_webhook_analytics(state, workspace_id, &job_payload, plan_code.as_deref());
    }

    Ok(serde_json::json!({"status": "processed"}))
}
