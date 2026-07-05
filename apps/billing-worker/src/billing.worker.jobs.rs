use serde_json::Value;

use crate::worker::{BillingWorkerState, email::process_billing_email_job};

#[path = "billing.worker.jobs.mollie.rs"]
mod mollie;
#[path = "billing.worker.jobs.stripe.rs"]
mod stripe;

const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(60);
const STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(600);
const BILLING_QUEUES: [&str; 3] = [
    nvbes_billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS,
    nvbes_billing::jobs::JOB_MOLLIE_WEBHOOK_PROCESS,
    nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND,
];

pub(crate) async fn recover_stale_jobs(
    redis: &nvbes_redis::RedisPool,
    observability: &nvbes_observability::metrics::HttpMetrics,
) -> anyhow::Result<()> {
    for queue in BILLING_QUEUES {
        for (job_type, outcome) =
            nvbes_redis::worker_queue::recover_stale_jobs(redis, queue, STALE_AFTER, RETRY_DELAY)
                .await?
        {
            observability.record_worker_queue_recovery(&job_type, &outcome);
        }
    }
    Ok(())
}

pub(crate) async fn claim_next_job(
    redis: &nvbes_redis::RedisPool,
) -> anyhow::Result<Option<nvbes_redis::worker_queue::QueuedJob>> {
    Ok(nvbes_redis::worker_queue::claim_next_job(redis, &BILLING_QUEUES, 5).await?)
}

pub(crate) async fn execute_job(
    state: &BillingWorkerState,
    job: &nvbes_redis::worker_queue::QueuedJob,
) -> anyhow::Result<Value> {
    match job.job_type.as_str() {
        nvbes_billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS => {
            stripe::process_stripe_webhook_job(state, job).await
        }
        nvbes_billing::jobs::JOB_MOLLIE_WEBHOOK_PROCESS => {
            mollie::process_mollie_webhook_job(state, job).await
        }
        nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND => process_billing_email_job(state, job).await,
        _ => Err(anyhow::anyhow!("Unknown billing job type: {}", job.job_type)),
    }
}

pub(crate) async fn mark_job_succeeded(
    redis: &nvbes_redis::RedisPool,
    job: &nvbes_redis::worker_queue::QueuedJob,
    result: Value,
) -> anyhow::Result<()> {
    nvbes_redis::worker_queue::mark_job_succeeded(redis, job, result).await?;
    Ok(())
}

pub(crate) async fn mark_job_failed(
    redis: &nvbes_redis::RedisPool,
    job: &nvbes_redis::worker_queue::QueuedJob,
    error: &str,
    retryable: bool,
) -> anyhow::Result<()> {
    let retry_delay = std::time::Duration::from_secs(60 * job.attempts.max(1) as u64);
    nvbes_redis::worker_queue::mark_job_failed(redis, job, error, retryable, retry_delay).await?;
    Ok(())
}

pub(crate) fn should_retry_job(job_type: &str, _error: &anyhow::Error) -> bool {
    matches!(
        job_type,
        nvbes_billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS
            | nvbes_billing::jobs::JOB_MOLLIE_WEBHOOK_PROCESS
            | nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND
    )
}

#[cfg(test)]
mod tests {
    use super::{BILLING_QUEUES, should_retry_job};

    #[test]
    fn billing_worker_queues_are_exclusively_billing_runtime() {
        assert_eq!(
            BILLING_QUEUES,
            [
                nvbes_billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS,
                nvbes_billing::jobs::JOB_MOLLIE_WEBHOOK_PROCESS,
                nvbes_billing::jobs::JOB_BILLING_EMAIL_SEND,
            ]
        );
        for queue in BILLING_QUEUES {
            assert!(queue.starts_with("billing."));
        }
    }

    #[test]
    fn billing_worker_retries_only_billing_runtime_jobs() {
        let error = anyhow::anyhow!("transient");
        for queue in BILLING_QUEUES {
            assert!(should_retry_job(queue, &error));
        }
        for non_billing_queue in [
            "email.send",
            "email.webhook.process",
            "data.export",
            "developer.webhook.deliver",
        ] {
            assert!(!should_retry_job(non_billing_queue, &error));
        }
    }
}
