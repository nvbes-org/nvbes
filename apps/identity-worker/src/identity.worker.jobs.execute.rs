use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;
use serde_json::Value;

use crate::app::AppState;
use crate::worker::job_failure::JobExecutionError;
use nvbes_redis::worker_queue::QueuedJob;

pub(super) async fn execute_job(
    state: &AppState,
    job: &QueuedJob,
) -> Result<Value, JobExecutionError> {
    match job.job_type.as_str() {
        JOB_EMAIL_SUBMIT => submit_email_job(state, job).await,
        _ => Err(JobExecutionError::permanent(
            "unknown_job_type",
            "Worker received an unsupported job type",
        )),
    }
}

async fn submit_email_job(state: &AppState, job: &QueuedJob) -> Result<Value, JobExecutionError> {
    let command: nvbes_email::EmailCommand =
        serde_json::from_value(job.payload.clone()).map_err(|_| {
            JobExecutionError::permanent(
                "invalid_email_command_payload",
                "Email command payload is invalid",
            )
        })?;
    if command.deliver_before <= chrono::Utc::now() {
        return Err(JobExecutionError::permanent(
            "email_command_expired",
            "Email command expired before global acceptance",
        ));
    }
    let receipt = state
        .email
        .send(command)
        .await
        .map_err(|error| JobExecutionError::from_email_client(&error))?;
    Ok(serde_json::json!({
        "status": "accepted",
        "message_id": receipt.message_id,
        "duplicate": receipt.duplicate,
    }))
}
