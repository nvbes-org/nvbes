use serde_json::Value;

use crate::app::AppState;
use nvbes_product_account::email::jobs::{
    JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
};

use nvbes_redis::worker_queue::QueuedJob;

use super::job_failure::JobExecutionError;

#[path = "identity.worker.jobs.email_delivery.rs"]
mod email_delivery;
#[path = "identity.worker.jobs.execute.rs"]
mod execute;
#[path = "identity.worker.jobs.process_data_export.rs"]
mod process_data_export;
#[path = "identity.worker.jobs.process_email_event.rs"]
mod process_email_event;

const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(60);
pub(super) const STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(600);

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

pub(crate) async fn renew_job_lease(
    redis: &nvbes_redis::RedisPool,
    job: &QueuedJob,
) -> anyhow::Result<()> {
    nvbes_redis::worker_queue::renew_job_lease(redis, job).await?;
    Ok(())
}

pub(super) fn should_retry_job(job_type: &str, error: &JobExecutionError) -> bool {
    matches!(
        job_type,
        JOB_EMAIL_SEND | JOB_EMAIL_WEBHOOK_PROCESS | JOB_DATA_EXPORT
    ) && error.is_retryable()
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

pub(super) async fn execute_job(
    state: &AppState,
    job: &QueuedJob,
) -> Result<Value, JobExecutionError> {
    execute::execute_job(state, job).await
}

pub(super) fn email_from_address(
    config: &nvbes_core::config::AppConfig,
) -> Result<nvbes_email::EmailAddress, JobExecutionError> {
    let email = match config.email_from_email.clone() {
        Some(email) => email,
        None if config.environment == "development" => "dev@nvbes.local".to_string(),
        None => {
            return Err(JobExecutionError::permanent(
                "email_from_missing",
                "Email sender address is not configured",
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::should_retry_job;
    use crate::worker::job_failure::JobExecutionError;
    use nvbes_product_account::email::jobs::{
        JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
    };

    #[test]
    fn identity_worker_retries_only_transient_owned_jobs() {
        let transient =
            JobExecutionError::transient("provider_unavailable", "Provider is unavailable");
        let permanent = JobExecutionError::permanent("invalid_payload", "Job payload is invalid");

        for job_type in [JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS, JOB_DATA_EXPORT] {
            assert!(
                should_retry_job(job_type, &transient),
                "account-worker should retry transient owned job {job_type}"
            );
            assert!(!should_retry_job(job_type, &permanent));
        }

        for job_type in [
            concat!("billing.", "stripe.webhook.process"),
            concat!("billing.", "mollie.webhook.process"),
            concat!("billing.", "email.send"),
        ] {
            assert!(
                !should_retry_job(job_type, &transient),
                "account-worker must not retry Billing job {job_type}"
            );
        }
    }
}
