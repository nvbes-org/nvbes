use serde_json::Value;

use crate::app::AppState;
use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;

use nvbes_redis::worker_queue::QueuedJob;

use super::job_failure::JobExecutionError;

#[path = "identity.worker.jobs.execute.rs"]
mod execute;

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
    job_type == JOB_EMAIL_SUBMIT && error.is_retryable()
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

#[cfg(test)]
#[path = "identity.worker.jobs.tests.rs"]
mod tests;
