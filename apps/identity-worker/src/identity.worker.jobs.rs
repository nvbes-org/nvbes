use anyhow::Context;
use serde_json::Value;
use sqlx::PgPool;

use crate::app::AppState;
use crate::domains::billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS;
use crate::domains::billing::webhooks::process_stripe_event;
use crate::email::jobs::{
    EmailSendPayload, JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
};
use crate::http::error::AppError;
use crate::worker::analytics::capture_billing_webhook_analytics;

use nvbes_redis::worker_queue::QueuedJob;

const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(60);
const STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(600);

pub(crate) async fn recover_stale_jobs(
    redis: &nvbes_redis::RedisPool,
    queue: &str,
    observability: &nvbes_observability::metrics::HttpMetrics,
) -> anyhow::Result<()> {
    for (job_type, outcome) in
        nvbes_redis::worker_queue::recover_stale_jobs(redis, queue, STALE_AFTER, RETRY_DELAY)
            .await?
    {
        observability.record_worker_queue_recovery(&job_type, &outcome);
    }
    Ok(())
}

pub(crate) async fn claim_next_job(
    redis: &nvbes_redis::RedisPool,
    queues: &[&str],
) -> anyhow::Result<Option<QueuedJob>> {
    Ok(nvbes_redis::worker_queue::claim_next_job(redis, queues, 5).await?)
}

pub(crate) async fn mark_job_succeeded(
    redis: &nvbes_redis::RedisPool,
    job: &QueuedJob,
    result: Value,
) -> anyhow::Result<()> {
    nvbes_redis::worker_queue::mark_job_succeeded(redis, job, result).await?;
    Ok(())
}

pub(crate) fn should_retry_job(job_type: &str, _error: &anyhow::Error) -> bool {
    matches!(
        job_type,
        JOB_EMAIL_SEND | JOB_EMAIL_WEBHOOK_PROCESS | JOB_STRIPE_WEBHOOK_PROCESS | JOB_DATA_EXPORT
    )
}

pub(crate) async fn mark_job_failed(
    redis: &nvbes_redis::RedisPool,
    job: &QueuedJob,
    error: &str,
    retryable: bool,
) -> anyhow::Result<()> {
    let retry_delay = std::time::Duration::from_secs(60 * job.attempts.max(1) as u64);
    nvbes_redis::worker_queue::mark_job_failed(redis, job, error, retryable, retry_delay).await?;
    Ok(())
}

pub(crate) async fn execute_job(state: &AppState, job: &QueuedJob) -> anyhow::Result<Value> {
    match job.job_type.as_str() {
        JOB_EMAIL_SEND => {
            let payload: EmailSendPayload = serde_json::from_value(job.payload.clone())?;
            let to_email = payload.to_email.clone();
            let subject = payload.subject.clone();
            let reply_to = state
                .config
                .scw_tem_from_email
                .clone()
                .unwrap_or_else(|| to_email.clone());

            let from_email = state
                .config
                .scw_tem_from_email
                .clone()
                .context("SCW_TEM_FROM_EMAIL must be set for email sending")?;

            let msg = nvbes_email::EmailMessage {
                from: nvbes_email::EmailAddress {
                    email: from_email,
                    name: Some(
                        state
                            .config
                            .scw_tem_from_name
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
        JOB_EMAIL_WEBHOOK_PROCESS => {
            process_email_event(&state.db, &job.payload)
                .await
                .map_err(|e| anyhow::anyhow!(format!("Email event processing failed: {e:?}")))?;
            Ok(serde_json::json!({"status": "processed"}))
        }
        JOB_STRIPE_WEBHOOK_PROCESS => {
            let job_payload: nvbes_billing::StripeWebhookEvent =
                serde_json::from_value(job.payload.clone())
                    .context("Invalid Stripe webhook job payload")?;
            let mut tx = state.db.begin().await?;
            let workspace_id = process_stripe_event(&mut tx, &job_payload)
                .await
                .map_err(|e| anyhow::anyhow!(format!("Stripe webhook processing failed: {e:?}")))?;
            tx.commit().await?;

            if let Some(workspace_id) = workspace_id {
                let _ = nvbes_redis::pubsub::publish_workspace_updated(
                    &state.redis,
                    &workspace_id.to_string(),
                )
                .await;

                let plan_code: Option<String> = sqlx::query_scalar(
                    "SELECT p.code FROM workspaces w JOIN plans p ON p.id = w.plan_id WHERE w.id = $1"
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

                capture_billing_webhook_analytics(
                    state,
                    workspace_id,
                    &job_payload,
                    plan_code.as_deref(),
                );
            }

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

async fn process_email_event(db: &PgPool, payload: &Value) -> Result<(), AppError> {
    let provider_event_id = payload
        .get("provider_event_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::bad_request("invalid_event_payload", "Missing provider_event_id")
        })?;
    let provider_email_id = payload
        .get("provider_email_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let email = payload
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::bad_request("invalid_event_payload", "Missing email"))?;
    let event_type = payload
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::bad_request("invalid_event_payload", "Missing event_type"))?;

    let msg_status = match event_type {
        "email_delivered" => Some("delivered"),
        "email_mailbox_not_found" => Some("bounced"),
        "email_bounced" | "email_blacklisted" => Some("bounced"),
        "email_spam" => Some("complained"),
        "email_dropped" => Some("dropped"),
        _ => None,
    };

    let mut tx = db.begin().await?;

    if let Some(status) = msg_status {
        if !provider_email_id.is_empty() {
            crate::email::db::update_email_message_status_by_provider_id_tx(
                &mut tx,
                provider_email_id,
                status,
            )
            .await?;
        }
    }

    match event_type {
        "email_mailbox_not_found" => {
            crate::email::db::suppress_email_tx(
                &mut tx,
                email,
                "hard_bounce",
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        "email_spam" | "email_bounced" | "email_blacklisted" => {
            crate::email::db::suppress_email_tx(
                &mut tx,
                email,
                if event_type == "email_spam" {
                    "spam_complaint"
                } else {
                    "bounced"
                },
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        "email_unsubscribed" => {
            crate::email::db::suppress_email_tx(
                &mut tx,
                email,
                "unsubscribed",
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        "email_dropped" => {
            crate::email::db::suppress_email_tx(
                &mut tx,
                email,
                "dropped",
                serde_json::json!({"event_type": event_type, "provider_event_id": provider_event_id}),
            )
            .await?;
        }
        _ => {}
    }

    crate::email::db::mark_email_event_processed_tx(&mut tx, provider_event_id).await?;
    tx.commit().await?;

    Ok(())
}

async fn process_data_export(state: &AppState, payload: &Value) -> anyhow::Result<()> {
    let user_id: uuid::Uuid = payload
        .get("user_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing user_id"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid user_id: {e}"))?;
    let email: String = payload
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing email"))
        .map(String::from)?;

    let user = crate::domains::auth::db::fetch_user_record(&state.db, user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user: {e:?}"))?;

    let export = crate::domains::auth::data_export::build_account_export(&state.db, user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build data export: {e:?}"))?;
    crate::domains::auth::data_export::store_account_export(&state.redis, user_id, &export)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store data export: {e:?}"))?;

    let html_body = format!(
        "<p>Bonjour {},</p><p>Votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.</p><p>L'equipe nvbes</p>",
        user.display_name
    );

    let from_email = state
        .config
        .scw_tem_from_email
        .clone()
        .ok_or_else(|| anyhow::anyhow!("SCW_TEM_FROM_EMAIL must be set"))?;

    let msg = nvbes_email::EmailMessage {
        from: nvbes_email::EmailAddress {
            email: from_email,
            name: Some(
                state
                    .config
                    .scw_tem_from_name
                    .clone()
                    .unwrap_or_else(|| "nvbes".to_string()),
            ),
        },
        to: vec![nvbes_email::EmailAddress {
            email: email.clone(),
            name: Some(user.display_name.clone()),
        }],
        subject: "Export de vos donnees - nvbes".to_string(),
        html_body: Some(html_body),
        text_body: Some(format!(
            "Bonjour {}, votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.",
            user.display_name
        )),
        headers: vec![("Reply-To".to_string(), email.clone())],
    };

    let result = state
        .email
        .send_message(&msg)
        .await
        .context("Failed to send export email")?;

    let mut tx = state.db.begin().await?;
    crate::email::db::record_email_sent_event_tx(
        &mut tx,
        &email,
        &result.provider_email_id,
        "Export de vos donnees - nvbes",
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to record sent event: {e:?}"))?;
    tx.commit().await?;

    Ok(())
}
