use std::time::{Duration, Instant};

use nvbes_observability::{
    WorkerJobContext, WorkerMonitorSchedule, capture_worker_heartbeat, capture_worker_job_error,
    metrics::HttpMetrics, worker_monitor_slug,
};
use tokio::time::{Duration as TokioDuration, sleep};

use crate::app::AppState;
use crate::domains::billing::jobs::JOB_STRIPE_WEBHOOK_PROCESS;
use crate::email::jobs::{JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS};

use super::jobs::{
    claim_next_job, execute_job, mark_job_failed, mark_job_succeeded, recover_stale_jobs,
    should_retry_job,
};

const WORKER_QUEUES: [&str; 4] = [
    JOB_EMAIL_SEND,
    JOB_EMAIL_WEBHOOK_PROCESS,
    JOB_STRIPE_WEBHOOK_PROCESS,
    JOB_DATA_EXPORT,
];
const SENTRY_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
const SENTRY_HEARTBEAT_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 5,
    checkin_margin_minutes: 2,
    max_runtime_minutes: 2,
};

pub async fn run_loop_until_shutdown<S>(state: AppState, shutdown: S) -> anyhow::Result<()>
where
    S: std::future::Future<Output = ()> + Send,
{
    let observability = state.observability.clone();
    let mut housekeeping_last_run = Instant::now() - Duration::from_secs(3600);
    let mut sentry_heartbeat_last_run = Instant::now() - SENTRY_HEARTBEAT_INTERVAL;
    tokio::pin!(shutdown);
    loop {
        capture_sentry_heartbeat_if_due(&state, &mut sentry_heartbeat_last_run);

        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            result = super::housekeeping::run_if_due(&state, &mut housekeeping_last_run) => result?,
        }
        let ran = tokio::select! {
            _ = &mut shutdown => return Ok(()),
            result = run_once(&state, &observability) => result?,
        };
        let sleep_for = if ran {
            TokioDuration::from_secs(1)
        } else {
            TokioDuration::from_secs(5)
        };
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            _ = sleep(sleep_for) => {},
        }
    }
}

pub async fn run_once(state: &AppState, observability: &HttpMetrics) -> anyhow::Result<bool> {
    for queue in WORKER_QUEUES {
        recover_stale_jobs(&state.redis, queue, observability).await?;
    }

    let Some(job) = claim_next_job(&state.redis, &WORKER_QUEUES).await? else {
        return Ok(false);
    };

    let started_at = Instant::now();
    let job_type = job.job_type.clone();
    let outcome = execute_job(state, &job).await;
    let duration = started_at.elapsed();

    match outcome {
        Ok(result) => {
            mark_job_succeeded(&state.redis, &job, result).await?;
            observability.record_worker_queue_job(&job_type, "success", duration);
        }
        Err(error) => {
            tracing::error!(error = %error, job_id = %job.id, job_type = %job_type, "Job failed");
            capture_worker_job_error(
                error.as_ref(),
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
            mark_job_failed(&state.redis, &job, &error.to_string(), retryable).await?;
            observability.record_worker_queue_job(&job_type, "failure", duration);
        }
    }
    Ok(true)
}

fn capture_sentry_heartbeat_if_due(state: &AppState, last_run: &mut Instant) {
    if last_run.elapsed() < SENTRY_HEARTBEAT_INTERVAL {
        return;
    }

    capture_worker_heartbeat(
        &state.config.environment,
        &worker_monitor_slug("identity-worker", "loop-heartbeat"),
        SENTRY_HEARTBEAT_SCHEDULE,
    );
    *last_run = Instant::now();
}
