use std::{
    error::Error,
    time::{Duration, Instant},
};

use nvbes_observability::{
    WorkerJobContext, WorkerMonitorSchedule, WorkerOperationContext, capture_worker_heartbeat,
    capture_worker_job_error, capture_worker_operation_error, metrics::HttpMetrics,
    worker_monitor_slug,
};
use tokio::time::{Duration as TokioDuration, sleep};

use crate::app::AppState;
use nvbes_product_account::email::jobs::{
    JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
};

use super::jobs::{
    claim_next_job, execute_job, mark_job_failed, mark_job_succeeded, recover_stale_jobs,
    should_retry_job,
};

const WORKER_QUEUES: [&str; 3] = [JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS, JOB_DATA_EXPORT];
const ACCESS_REVIEW_SCHEDULE_INTERVAL: Duration = Duration::from_secs(900);
const ACCESS_REVIEW_REMINDER_INTERVAL: Duration = Duration::from_secs(3600);
const WORKER_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
const TRANSIENT_INFRA_ERROR_SLEEP: TokioDuration = TokioDuration::from_secs(5);
const WORKER_HEARTBEAT_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 5,
    checkin_margin_minutes: 2,
    max_runtime_minutes: 2,
};

pub async fn run_loop_until_shutdown<S>(state: AppState, shutdown: S) -> anyhow::Result<()>
where
    S: std::future::Future<Output = ()> + Send,
{
    let observability = state.observability.clone();
    let mut access_review_schedule_last_run = Instant::now() - ACCESS_REVIEW_SCHEDULE_INTERVAL;
    let mut access_review_reminder_last_run = Instant::now() - ACCESS_REVIEW_REMINDER_INTERVAL;
    let mut housekeeping_last_run = Instant::now() - Duration::from_secs(3600);
    let mut worker_heartbeat_last_run = Instant::now() - WORKER_HEARTBEAT_INTERVAL;
    tokio::pin!(shutdown);
    loop {
        capture_worker_heartbeat_if_due(&state, &mut worker_heartbeat_last_run);
        if let Err(error) =
            run_access_review_schedules_if_due(&state, &mut access_review_schedule_last_run).await
        {
            capture_loop_error(&state, "access_review_schedules", error.as_ref());
            return Err(error);
        }
        if let Err(error) =
            run_access_review_reminders_if_due(&state, &mut access_review_reminder_last_run).await
        {
            capture_loop_error(&state, "access_review_reminders", error.as_ref());
            return Err(error);
        }
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            result = super::housekeeping::run_if_due(&state, &mut housekeeping_last_run) => {
                if let Err(error) = result {
                    capture_loop_error(&state, "housekeeping", error.as_ref());
                    tracing::warn!(%error, "account worker housekeeping failed; retrying after backoff");
                    sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
                    continue;
                }
            },
        }
        let ran = tokio::select! {
            _ = &mut shutdown => return Ok(()),
            result = run_once(&state, &observability) => {
                match result {
                    Ok(ran) => ran,
                    Err(error) => {
                        capture_loop_error(&state, "worker_loop", error.as_ref());
                        tracing::warn!(%error, "account worker loop failed; retrying after backoff");
                        sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
                        continue;
                    }
                }
            },
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

async fn run_access_review_reminders_if_due(
    state: &AppState,
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < ACCESS_REVIEW_REMINDER_INTERVAL {
        return Ok(());
    }
    let run = nvbes_product_account::enterprise::access_reviews::scheduler::enqueue_due_campaign_reminders(
        &state.db,
        &state.redis,
        &state.config,
    )
    .await
    .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.message))?;
    if run.reminders_enqueued > 0 {
        tracing::info!(
            reminders_enqueued = run.reminders_enqueued,
            "enqueued access review reminders"
        );
    }
    *last_run = Instant::now();
    Ok(())
}

async fn run_access_review_schedules_if_due(
    state: &AppState,
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < ACCESS_REVIEW_SCHEDULE_INTERVAL {
        return Ok(());
    }
    let run =
        nvbes_product_account::enterprise::access_reviews::scheduler::materialize_due_schedules(
            &state.db,
        )
        .await
        .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.message))?;
    if run.campaigns_created > 0 || run.empty_schedules > 0 {
        tracing::info!(
            campaigns_created = run.campaigns_created,
            empty_schedules = run.empty_schedules,
            "materialized due access review schedules"
        );
    }
    *last_run = Instant::now();
    Ok(())
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
                    app_name: "account-worker",
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

fn capture_loop_error(
    state: &AppState,
    operation: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) {
    capture_worker_operation_error(
        error,
        &WorkerOperationContext {
            app_name: "account-worker",
            environment: &state.config.environment,
            operation,
        },
    );
}

fn capture_worker_heartbeat_if_due(state: &AppState, last_run: &mut Instant) {
    if last_run.elapsed() < WORKER_HEARTBEAT_INTERVAL {
        return;
    }

    capture_worker_heartbeat(
        &state.config.environment,
        &worker_monitor_slug("account-worker", "loop-heartbeat"),
        WORKER_HEARTBEAT_SCHEDULE,
    );
    *last_run = Instant::now();
}

#[cfg(test)]
mod tests {
    use super::WORKER_QUEUES;
    use nvbes_product_account::email::jobs::{
        JOB_DATA_EXPORT, JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS,
    };

    #[test]
    fn identity_worker_queues_exclude_billing_runtime() {
        assert_eq!(
            WORKER_QUEUES,
            [JOB_EMAIL_SEND, JOB_EMAIL_WEBHOOK_PROCESS, JOB_DATA_EXPORT]
        );
        for queue in WORKER_QUEUES {
            assert!(
                !queue.starts_with("billing."),
                "account-worker must not claim Billing queue {queue}"
            );
        }
    }
}
