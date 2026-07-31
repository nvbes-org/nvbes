use nvbes_product_identity::email::jobs::{
    EmailSendPayload, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
};
use serde_json::Value;

use crate::app::AppState;
use crate::worker::job_failure::JobExecutionError;

use nvbes_redis::worker_queue::QueuedJob;

use super::{
    email_delivery::deliver_email, email_from_address, process_email_event::process_email_event,
};

pub(super) async fn execute_job(
    state: &AppState,
    job: &QueuedJob,
) -> Result<Value, JobExecutionError> {
    match job.job_type.as_str() {
        JOB_EMAIL_SEND => send_email_job(state, job).await,
        JOB_EMAIL_WEBHOOK_PROCESS => {
            process_email_event(&state.db, &job.payload).await?;
            Ok(serde_json::json!({"status": "processed"}))
        }
        _ => Err(JobExecutionError::permanent(
            "unknown_job_type",
            "Worker received an unsupported job type",
        )),
    }
}

async fn send_email_job(state: &AppState, job: &QueuedJob) -> Result<Value, JobExecutionError> {
    let payload: EmailSendPayload = serde_json::from_value(job.payload.clone()).map_err(|_| {
        JobExecutionError::permanent("invalid_email_job_payload", "Email job payload is invalid")
    })?;
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
        headers: vec![("Reply-To".to_string(), reply_to)],
    };
    let outcome = deliver_email(
        &state.db,
        state.email.as_ref(),
        job.id,
        &payload.business_type,
        &to_email,
        &subject,
        message,
    )
    .await?;

    Ok(serde_json::json!({
        "status": "sent",
        "provider_email_id": outcome.provider_email_id,
        "deduplicated": outcome.deduplicated,
    }))
}
