use nvbes_observability::metrics::HttpMetrics;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::db::Database;
use nvbes_observability::{
    WorkerJobContext, WorkerMonitorSchedule, capture_worker_heartbeat, capture_worker_job_error,
    worker_monitor_slug,
};

const WORKER_QUEUES: [&str; 9] = [
    super::super::maintenance::JOB_UPLOADS_PURGE_EXPIRED,
    super::super::maintenance::JOB_QUOTAS_RECALCULATE,
    super::super::maintenance::JOB_TRASH_PURGE,
    super::super::maintenance::JOB_STORAGE_PURGE_DELETED,
    super::super::maintenance::JOB_STORAGE_PURGE_QUARANTINED,
    super::super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE,
    super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE,
    super::super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT,
    super::super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT,
];
const SENTRY_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
const TRANSIENT_INFRA_ERROR_SLEEP: Duration = Duration::from_secs(5);
const SENTRY_HEARTBEAT_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 5,
    checkin_margin_minutes: 2,
    max_runtime_minutes: 2,
};

pub async fn run_once(
    database: &Database,
    redis: &nvbes_redis::RedisPool,
    storage: Arc<dyn nvbes_storage::ObjectStore>,
    observability: &HttpMetrics,
) -> anyhow::Result<bool> {
    for queue in WORKER_QUEUES {
        super::super::db::recover_stale_jobs(redis, queue, observability).await?;
    }

    let Some(job) = super::super::db::claim_next_job(redis, &WORKER_QUEUES).await? else {
        return Ok(false);
    };

    observability.record_postgres_pool(
        "worker",
        &std::env::var("NVBES_ENVIRONMENT").unwrap_or_default(),
        database.size(),
        database.num_idle(),
    );

    let result = super::dispatch::execute_job(&job, database, storage.as_ref()).await;

    match result {
        Ok(value) => {
            super::super::db::mark_job_succeeded(redis, &job, value).await?;
        }
        Err(err) => {
            tracing::error!(job_id = %job.id, error = %err, "job failed");
            let environment = worker_environment();
            capture_worker_job_error(
                err.as_ref(),
                &WorkerJobContext {
                    app_name: "drive-worker",
                    environment: &environment,
                    queue: &job.queue,
                    job_type: &job.job_type,
                    job_id: job.id,
                    attempts: job.attempts,
                    max_attempts: job.max_attempts,
                },
            );
            super::super::db::mark_job_failed(redis, &job, &err.to_string(), true).await?;
        }
    }

    Ok(true)
}

pub async fn run_loop(
    database: &Database,
    redis: &nvbes_redis::RedisPool,
    storage: Arc<dyn nvbes_storage::ObjectStore>,
    observability: &HttpMetrics,
) -> anyhow::Result<()> {
    let db_clone = database.clone();
    let redis_clone = redis.clone();
    let environment = worker_environment();
    let mut sentry_heartbeat_last_run = Instant::now() - SENTRY_HEARTBEAT_INTERVAL;
    tokio::spawn(async move {
        loop {
            if let Err(error) =
                super::pubsub::run_pubsub_listener(db_clone.clone(), redis_clone.clone()).await
            {
                tracing::warn!(%error, "Redis PubSub listener failed; restarting after backoff");
                tokio::time::sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
                continue;
            }

            tracing::warn!("Redis PubSub listener exited; restarting after backoff");
            tokio::time::sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
        }
    });

    loop {
        capture_sentry_heartbeat_if_due(&environment, &mut sentry_heartbeat_last_run);

        let processed = match run_once(database, redis, storage.clone(), observability).await {
            Ok(processed) => processed,
            Err(error) => {
                tracing::warn!(%error, "drive worker loop failed; retrying after backoff");
                tokio::time::sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
                continue;
            }
        };

        if !processed {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

fn capture_sentry_heartbeat_if_due(environment: &str, last_run: &mut Instant) {
    if last_run.elapsed() < SENTRY_HEARTBEAT_INTERVAL {
        return;
    }

    capture_worker_heartbeat(
        environment,
        &worker_monitor_slug("drive-worker", "loop-heartbeat"),
        SENTRY_HEARTBEAT_SCHEDULE,
    );
    *last_run = Instant::now();
}

fn worker_environment() -> String {
    std::env::var("NVBES_ENVIRONMENT").unwrap_or_default()
}
