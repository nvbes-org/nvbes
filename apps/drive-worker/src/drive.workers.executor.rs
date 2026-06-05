use nvbes_observability::metrics::HttpMetrics;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::db::Database;
use nvbes_observability::{
    WorkerJobContext, WorkerMonitorSchedule, capture_worker_heartbeat, capture_worker_job_error,
    worker_monitor_slug,
};
use nvbes_redis::worker_queue::QueuedJob;

const WORKER_QUEUES: [&str; 9] = [
    super::maintenance::JOB_UPLOADS_PURGE_EXPIRED,
    super::maintenance::JOB_QUOTAS_RECALCULATE,
    super::maintenance::JOB_TRASH_PURGE,
    super::maintenance::JOB_STORAGE_PURGE_DELETED,
    super::maintenance::JOB_STORAGE_PURGE_QUARANTINED,
    super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE,
    super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE,
    super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT,
    super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT,
];
const SENTRY_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
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
        super::db::recover_stale_jobs(redis, queue, observability).await?;
    }

    let Some(job) = super::db::claim_next_job(redis, &WORKER_QUEUES).await? else {
        return Ok(false);
    };

    observability.record_postgres_pool(
        "worker",
        &std::env::var("NVBES_ENVIRONMENT").unwrap_or_default(),
        database.size(),
        database.num_idle(),
    );

    let result = execute_job(&job, database, storage.as_ref()).await;

    match result {
        Ok(value) => {
            super::db::mark_job_succeeded(redis, &job, value).await?;
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
            super::db::mark_job_failed(redis, &job, &err.to_string(), true).await?;
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
        if let Err(e) = run_pubsub_listener(db_clone, redis_clone).await {
            tracing::error!("Redis PubSub listener failed: {:?}", e);
        }
    });

    loop {
        capture_sentry_heartbeat_if_due(&environment, &mut sentry_heartbeat_last_run);

        let processed = run_once(database, redis, storage.clone(), observability).await?;

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

async fn run_pubsub_listener(
    database: Database,
    redis: nvbes_redis::RedisPool,
) -> anyhow::Result<()> {
    use futures_util::StreamExt;

    let url =
        std::env::var("NVBES_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let client = redis::Client::open(url)?;
    let mut pubsub = client.get_async_pubsub().await?;

    pubsub.subscribe("nvbes:pubsub:workspace:deleted").await?;
    pubsub
        .subscribe("nvbes:pubsub:workspace:plan_updated")
        .await?;
    pubsub.subscribe("nvbes:pubsub:user:suspended").await?;
    pubsub.subscribe("nvbes:pubsub:session:revoked").await?;

    tracing::info!("Subscribed to real-time events on Redis PubSub");

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let channel = msg.get_channel_name();
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to get payload from PubSub message: {:?}", e);
                continue;
            }
        };
        tracing::info!(channel = %channel, payload = %payload, "Received real-time event via PubSub");

        if channel == "nvbes:pubsub:workspace:deleted" {
            if let Ok(workspace_id) = uuid::Uuid::parse_str(&payload) {
                tracing::info!(workspace_id = %workspace_id, "Handling workspace deleted event");
                if let Err(e) = nvbes_redis::worker_queue::enqueue_job(
                    &redis,
                    super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE,
                    super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE,
                    serde_json::json!({ "workspace_id": workspace_id }),
                    Some(&format!("pubsub:workspace_delete:{}", workspace_id)),
                    3,
                    true,
                    None,
                )
                .await
                {
                    tracing::error!("Failed to enqueue workspace deletion job: {:?}", e);
                }
            }
        } else if channel == "nvbes:pubsub:workspace:plan_updated" {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&payload) {
                if let (Some(workspace_id_str), Some(plan_code)) = (
                    val.get("workspace_id").and_then(|v| v.as_str()),
                    val.get("plan_code").and_then(|v| v.as_str()),
                ) {
                    if let Ok(workspace_id) = uuid::Uuid::parse_str(workspace_id_str) {
                        tracing::info!(workspace_id = %workspace_id, plan_code = %plan_code, "Handling workspace plan updated event");
                        let database_clone = database.clone();
                        let plan_code_owned = plan_code.to_owned();
                        tokio::spawn(async move {
                            match database_clone.begin().await {
                                Ok(mut tx) => {
                                    match nvbes_drive_api::domains::billing::db::plan_id_by_code_tx(&mut tx, &plan_code_owned).await {
                                        Ok(plan_id) => {
                                            if let Err(e) = nvbes_drive_api::domains::billing::db::project_workspace_plan_tx(&mut tx, workspace_id, plan_id).await {
                                                tracing::error!("Failed to project workspace plan: {:?}", e);
                                            } else if let Err(e) = tx.commit().await {
                                                tracing::error!("Failed to commit transaction: {:?}", e);
                                            } else {
                                                tracing::info!(workspace_id = %workspace_id, plan_code = %plan_code_owned, "Workspace plan successfully projected locally");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to fetch plan id by code: {:?}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to start transaction: {:?}", e);
                                }
                            }
                        });
                    }
                }
            }
        } else if channel == "nvbes:pubsub:user:suspended" {
            if let Ok(user_id) = uuid::Uuid::parse_str(&payload) {
                tracing::info!(user_id = %user_id, "Handling user suspended event");
                if let Err(e) = sqlx::query(
                    "UPDATE users SET status = 'suspended', updated_at = NOW() WHERE id = $1",
                )
                .bind(user_id)
                .execute(&*database)
                .await
                {
                    tracing::error!("Failed to suspend user in DB: {:?}", e);
                }
            }
        } else if channel == "nvbes:pubsub:session:revoked" {
            let parts: Vec<&str> = payload.split(':').collect();
            if parts.len() == 2 {
                if let (Ok(user_id), Ok(session_id)) = (
                    uuid::Uuid::parse_str(parts[0]),
                    uuid::Uuid::parse_str(parts[1]),
                ) {
                    tracing::info!(user_id = %user_id, session_id = %session_id, "Handling session revoked event");
                    if let Err(e) = sqlx::query(
                        r#"
                        INSERT INTO sessions (id, user_id, expires_at, revoked_at)
                        VALUES ($1, $2, NOW(), NOW())
                        ON CONFLICT (id) DO UPDATE SET revoked_at = NOW()
                        "#,
                    )
                    .bind(session_id)
                    .bind(user_id)
                    .execute(&*database)
                    .await
                    {
                        tracing::error!("Failed to revoke session in SQL DB: {:?}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn execute_job(
    job: &QueuedJob,
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<serde_json::Value> {
    if !is_known_job_type(job.job_type.as_str()) {
        anyhow::bail!("unknown job type: {}", job.job_type);
    }

    match job.job_type.as_str() {
        super::maintenance::JOB_UPLOADS_PURGE_EXPIRED => {
            super::maintenance::purge_expired_uploads(database, storage).await
        }
        super::maintenance::JOB_QUOTAS_RECALCULATE => {
            super::maintenance::recalculate_quotas(database).await
        }
        super::maintenance::JOB_TRASH_PURGE => super::maintenance::purge_trash(database).await,
        super::maintenance::JOB_STORAGE_PURGE_DELETED => {
            super::maintenance::storage::purge_deleted_storage(database, storage).await
        }
        super::maintenance::JOB_STORAGE_PURGE_QUARANTINED => {
            super::maintenance::storage::purge_quarantined(database, storage, 30).await
        }
        super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE => {
            super::privacy::delete::delete_account_data(job, database).await
        }
        super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE => {
            super::privacy::delete::delete_workspace_data(job, database, storage).await
        }
        super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT => {
            super::privacy::export::export_account_data(job, database, storage).await
        }
        super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT => {
            super::privacy::export::export_workspace_data(job, database, storage).await
        }
        _ => unreachable!("job type was already validated"),
    }
}

fn is_known_job_type(job_type: &str) -> bool {
    matches!(
        job_type,
        super::maintenance::JOB_UPLOADS_PURGE_EXPIRED
            | super::maintenance::JOB_QUOTAS_RECALCULATE
            | super::maintenance::JOB_TRASH_PURGE
            | super::maintenance::JOB_STORAGE_PURGE_DELETED
            | super::maintenance::JOB_STORAGE_PURGE_QUARANTINED
            | super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE
            | super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE
            | super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT
            | super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT
    )
}

#[cfg(test)]
mod tests {
    use super::is_known_job_type;

    #[test]
    fn known_privacy_job_types_are_accepted() {
        assert!(is_known_job_type(
            super::super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE
        ));
        assert!(is_known_job_type(
            super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE
        ));
        assert!(is_known_job_type(
            super::super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT
        ));
        assert!(is_known_job_type(
            super::super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT
        ));
    }

    #[test]
    fn unknown_job_types_are_rejected() {
        assert!(!is_known_job_type("unknown.job"));
    }
}
