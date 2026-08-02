use std::time::Instant;

use nvbes_observability::{WorkerJobContext, capture_worker_job_error, metrics::HttpMetrics};
use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;
use nvbes_redis::worker_queue::QueuedJob;
use tokio::time::{Instant as TokioInstant, MissedTickBehavior, interval_at};
use tracing::Instrument;

use crate::app::AppState;

use super::jobs::{
    claim_next_job, execute_job, mark_job_failed, mark_job_succeeded, recover_stale_jobs,
    renew_job_lease, should_retry_job,
};

const JOB_LEASE_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

pub(super) const WORKER_QUEUES: [&str; 1] = [JOB_EMAIL_SUBMIT];

pub(super) async fn claim_available_job(
    state: &AppState,
    observability: &HttpMetrics,
) -> anyhow::Result<Option<QueuedJob>> {
    for queue in WORKER_QUEUES {
        recover_stale_jobs(&state.redis, queue, observability).await?;
    }
    claim_next_job(&state.redis, &WORKER_QUEUES).await
}

pub(super) async fn process_claimed_job(
    state: &AppState,
    observability: &HttpMetrics,
    job: QueuedJob,
) -> anyhow::Result<()> {
    let started_at = Instant::now();
    let job_type = job.job_type.clone();
    let job_span = tracing::info_span!(
        "worker.job",
        otel.name = %format!("process {}", job_type),
        otel.kind = "consumer",
        otel.status_code = tracing::field::Empty,
        messaging.system = "redis",
        messaging.destination.name = %job.queue,
        messaging.operation.type = "process",
        job.type = %job_type,
        job.outcome = tracing::field::Empty,
    );
    let execution = execute_job(state, &job).instrument(job_span.clone());
    tokio::pin!(execution);
    let mut heartbeat = interval_at(
        TokioInstant::now() + JOB_LEASE_HEARTBEAT_INTERVAL,
        JOB_LEASE_HEARTBEAT_INTERVAL,
    );
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let outcome = loop {
        tokio::select! {
            biased;
            result = &mut execution => break result,
            _ = heartbeat.tick() => {
                if renew_job_lease(&state.redis, &job).await.is_err() {
                    tracing::warn!(
                        job_id = %job.id,
                        job_type = %job_type,
                        "Job lease renewal failed; in-flight effect continues under its durable ledger"
                    );
                }
            },
        }
    };
    let duration = started_at.elapsed();

    match outcome {
        Ok(result) => {
            mark_job_succeeded(&state.redis, &job, result).await?;
            job_span.record("job.outcome", "success");
            observability.record_worker_queue_job(&job_type, "success", duration);
        }
        Err(error) => {
            tracing::error!(
                error_class = error.class().as_str(),
                error_code = error.code(),
                job_id = %job.id,
                job_type = %job_type,
                "Job failed"
            );
            capture_worker_job_error(
                &error,
                &WorkerJobContext {
                    app_name: "identity-worker",
                    environment: &state.config.environment,
                    queue: &job.queue,
                    job_type: &job_type,
                    job_id: job.id,
                    attempts: job.attempts,
                    max_attempts: job.max_attempts,
                },
            );
            let retryable = should_retry_job(&job_type, &error);
            let safe_error = error.to_string();
            mark_job_failed(&state.redis, &job, &safe_error, retryable).await?;
            let outcome = failed_job_outcome(job.attempts, job.max_attempts, retryable);
            job_span.record("otel.status_code", "ERROR");
            job_span.record("job.outcome", outcome);
            observability.record_worker_queue_job(&job_type, outcome, duration);
        }
    }
    Ok(())
}

fn failed_job_outcome(attempts: u32, max_attempts: u32, retryable: bool) -> &'static str {
    if retryable && attempts < max_attempts {
        "retry_scheduled"
    } else {
        "dead_letter"
    }
}

#[cfg(test)]
mod tests {
    use super::{JOB_LEASE_HEARTBEAT_INTERVAL, WORKER_QUEUES, failed_job_outcome};
    use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;

    #[test]
    fn identity_worker_queues_exclude_account_and_billing_runtime() {
        assert_eq!(WORKER_QUEUES, [JOB_EMAIL_SUBMIT]);
        for queue in WORKER_QUEUES {
            assert!(
                !queue.starts_with("billing."),
                "identity-worker must not claim Billing queue {queue}"
            );
            assert!(
                !queue.starts_with("account."),
                "identity-worker must not claim Account queue {queue}"
            );
        }
    }

    #[test]
    fn failed_job_outcome_matches_retry_budget() {
        assert_eq!(failed_job_outcome(1, 3, true), "retry_scheduled");
        assert_eq!(failed_job_outcome(3, 3, true), "dead_letter");
        assert_eq!(failed_job_outcome(1, 3, false), "dead_letter");
    }

    #[test]
    fn lease_heartbeat_precedes_stale_recovery_window() {
        assert!(
            JOB_LEASE_HEARTBEAT_INTERVAL.as_secs() * 3 < super::super::jobs::STALE_AFTER.as_secs()
        );
    }
}
