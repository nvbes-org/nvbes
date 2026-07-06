use std::{
    error::Error,
    time::{Duration, Instant},
};

use nvbes_observability::{
    WorkerJobContext, WorkerMonitorSchedule, WorkerOperationContext, capture_worker_heartbeat,
    capture_worker_job_error, capture_worker_operation_error, worker_monitor_slug,
};
use tokio::time::{Duration as TokioDuration, sleep};

use super::BillingWorkerState;

const BILLING_DUNNING_INTERVAL: Duration = Duration::from_secs(300);
const BILLING_DUNNING_BATCH_SIZE: i64 = 100;
const BILLING_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(3600);
const WORKER_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
const TRANSIENT_INFRA_ERROR_SLEEP: TokioDuration = TokioDuration::from_secs(5);
const WORKER_HEARTBEAT_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 5,
    checkin_margin_minutes: 2,
    max_runtime_minutes: 2,
};

pub async fn run_loop_until_shutdown<S>(
    state: BillingWorkerState,
    shutdown: S,
) -> anyhow::Result<()>
where
    S: std::future::Future<Output = ()> + Send,
{
    let mut billing_dunning_last_run = Instant::now() - BILLING_DUNNING_INTERVAL;
    let mut billing_reconciliation_last_run = Instant::now() - BILLING_RECONCILIATION_INTERVAL;
    let mut worker_heartbeat_last_run = Instant::now() - WORKER_HEARTBEAT_INTERVAL;
    tokio::pin!(shutdown);

    loop {
        capture_worker_heartbeat_if_due(&state, &mut worker_heartbeat_last_run);
        if let Err(error) = run_billing_dunning_if_due(&state, &mut billing_dunning_last_run).await
        {
            capture_loop_error(&state, "billing_dunning", error.as_ref());
            tracing::warn!(%error, "billing worker dunning failed; retrying after backoff");
            sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
            continue;
        }
        if let Err(error) =
            run_billing_reconciliation_if_due(&state, &mut billing_reconciliation_last_run).await
        {
            capture_loop_error(&state, "billing_reconciliation", error.as_ref());
            tracing::warn!(%error, "billing worker reconciliation failed; retrying after backoff");
            sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
            continue;
        }

        let ran = tokio::select! {
            _ = &mut shutdown => return Ok(()),
            result = run_once(&state) => {
                match result {
                    Ok(ran) => ran,
                    Err(error) => {
                        capture_loop_error(&state, "billing_worker_loop", error.as_ref());
                        tracing::warn!(%error, "billing worker loop failed; retrying after backoff");
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

pub async fn run_once(state: &BillingWorkerState) -> anyhow::Result<bool> {
    super::jobs::recover_stale_jobs(&state.redis, &state.observability).await?;

    let Some(job) = super::jobs::claim_next_job(&state.redis).await? else {
        return Ok(false);
    };

    let started_at = Instant::now();
    let job_type = job.job_type.clone();
    let outcome = super::jobs::execute_job(state, &job).await;
    let duration = started_at.elapsed();

    match outcome {
        Ok(result) => {
            super::jobs::mark_job_succeeded(&state.redis, &job, result).await?;
            state
                .observability
                .record_worker_queue_job(&job_type, "success", duration);
        }
        Err(error) => {
            tracing::error!(error = %error, job_id = %job.id, job_type = %job_type, "Billing job failed");
            capture_worker_job_error(
                error.as_ref(),
                &WorkerJobContext {
                    app_name: "billing-worker",
                    environment: &state.config.environment,
                    queue: &job.queue,
                    job_type: &job_type,
                    job_id: job.id,
                    attempts: job.attempts,
                    max_attempts: job.max_attempts,
                },
            );
            let retryable = super::jobs::should_retry_job(&job_type, &error);
            super::jobs::mark_job_failed(&state.redis, &job, &error.to_string(), retryable).await?;
            state
                .observability
                .record_worker_queue_job(&job_type, "failure", duration);
        }
    }

    Ok(true)
}

async fn run_billing_dunning_if_due(
    state: &BillingWorkerState,
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < BILLING_DUNNING_INTERVAL {
        return Ok(());
    }
    let started_at = Instant::now();
    let run = nvbes_billing::dunning_jobs::process_due_dunning_attempts(
        &state.db,
        BILLING_DUNNING_BATCH_SIZE,
    )
    .await?;
    state
        .observability
        .record_billing_operation("dunning", "success", started_at.elapsed());
    if run.attempts_processed > 0 {
        tracing::info!(
            attempts_processed = run.attempts_processed,
            "billing dunning attempts processed"
        );
    }
    *last_run = Instant::now();
    Ok(())
}

async fn run_billing_reconciliation_if_due(
    state: &BillingWorkerState,
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < BILLING_RECONCILIATION_INTERVAL {
        return Ok(());
    }
    let run =
        nvbes_billing::reconciliation_db::run_ledger_reconciliation(&state.db, chrono::Utc::now())
            .await?;
    if run.differences_created > 0 {
        tracing::warn!(
            run_id = %run.run_id,
            differences_created = run.differences_created,
            "billing reconciliation detected ledger differences"
        );
    } else {
        tracing::info!(run_id = %run.run_id, "billing reconciliation completed");
    }
    *last_run = Instant::now();
    Ok(())
}

fn capture_worker_heartbeat_if_due(state: &BillingWorkerState, last_run: &mut Instant) {
    if last_run.elapsed() < WORKER_HEARTBEAT_INTERVAL {
        return;
    }
    capture_worker_heartbeat(
        &worker_monitor_slug("billing-worker", "main_loop"),
        &state.config.environment,
        WORKER_HEARTBEAT_SCHEDULE,
    );
    *last_run = Instant::now();
}

fn capture_loop_error(
    state: &BillingWorkerState,
    operation: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) {
    capture_worker_operation_error(
        error,
        &WorkerOperationContext {
            app_name: "billing-worker",
            environment: &state.config.environment,
            operation,
        },
    );
    state
        .observability
        .record_billing_operation(operation, "failure", Duration::ZERO);
}
