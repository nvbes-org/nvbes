use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::logic::retry_backoff;
use crate::db::Database;
use nvbes_redis::worker_queue::QueuedJob;

pub async fn enqueue_job(
    redis: &nvbes_redis::RedisPool,
    queue: &str,
    job_type: &str,
    idempotency_key: &str,
    payload: serde_json::Value,
    max_attempts: i32,
) -> anyhow::Result<Uuid> {
    Ok(nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: queue.to_string(),
            job_type: job_type.to_string(),
            payload,
            idempotency_key: Some(idempotency_key.to_string()),
            max_attempts: max_attempts as u32,
            overwrite_terminal: true,
            job_id: None,
        },
    )
    .await?)
}

pub async fn recover_stale_jobs(
    redis: &nvbes_redis::RedisPool,
    queue: &str,
    observability: &nvbes_observability::metrics::HttpMetrics,
) -> anyhow::Result<()> {
    for (job_type, outcome) in nvbes_redis::worker_queue::recover_stale_jobs(
        redis,
        queue,
        std::time::Duration::from_secs(15 * 60),
        std::time::Duration::from_secs(60),
    )
    .await?
    {
        observability.record_worker_queue_recovery(&job_type, &outcome);
    }
    Ok(())
}

pub async fn claim_next_job(
    redis: &nvbes_redis::RedisPool,
    queues: &[&str],
) -> anyhow::Result<Option<QueuedJob>> {
    Ok(nvbes_redis::worker_queue::claim_next_job(redis, queues, 5).await?)
}

pub async fn mark_job_succeeded(
    redis: &nvbes_redis::RedisPool,
    job: &QueuedJob,
    result: serde_json::Value,
) -> anyhow::Result<()> {
    nvbes_redis::worker_queue::mark_job_succeeded(redis, job, result).await?;
    Ok(())
}

pub async fn mark_job_failed(
    redis: &nvbes_redis::RedisPool,
    job: &QueuedJob,
    error: &str,
    retryable: bool,
) -> anyhow::Result<()> {
    let retry_delay = retry_backoff(job.attempts as i32).max(std::time::Duration::from_secs(1));
    nvbes_redis::worker_queue::mark_job_failed(redis, job, error, retryable, retry_delay).await?;
    Ok(())
}

pub async fn mark_privacy_request_processing(
    database: &Database,
    request_id: Uuid,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE privacy_requests
        SET status = 'processing', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .execute(&**database)
    .await?;

    Ok(())
}

pub async fn mark_privacy_request_completed_tx(
    tx: &mut Transaction<'_, Postgres>,
    request_id: Uuid,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE privacy_requests
        SET status = 'completed', completed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn mark_privacy_request_rejected_tx(
    tx: &mut Transaction<'_, Postgres>,
    request_id: Uuid,
    error: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE privacy_requests
        SET status = 'failed', last_error = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .bind(error)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub fn row_to_json(_row: sqlx::postgres::PgRow) -> serde_json::Value {
    serde_json::Value::Null
}

pub fn rows_to_json(rows: Vec<sqlx::postgres::PgRow>) -> Vec<serde_json::Value> {
    rows.into_iter().map(|_| serde_json::Value::Null).collect()
}

pub async fn insert_system_audit_event(
    database: &Database,
    workspace_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (workspace_id, actor_user_id, action, target_type, target_id, metadata)
        VALUES ($1, NULL, $2, $3, $4, $5)
        "#,
    )
    .bind(workspace_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(&metadata)
    .execute(&**database)
    .await?;

    Ok(())
}

pub async fn compute_delete_impact(
    database: &Database,
    workspace_id: Uuid,
    object_id: Uuid,
) -> anyhow::Result<super::types::DeleteImpact> {
    let row = sqlx::query(
        r#"
        WITH RECURSIVE subtree AS (
          SELECT id, object_type, size_bytes
          FROM storage_objects
          WHERE workspace_id = $1 AND id = $2
          UNION ALL
          SELECT child.id, child.object_type, child.size_bytes
          FROM storage_objects child
          INNER JOIN subtree parent_tree ON child.parent_id = parent_tree.id
          WHERE child.workspace_id = $1
        )
        SELECT
          COUNT(*)::bigint AS deleted_object_count,
          COUNT(*) FILTER (WHERE object_type = 'file')::bigint AS deleted_file_count,
          COALESCE(SUM(size_bytes) FILTER (WHERE object_type = 'file'), 0)::bigint AS released_storage_bytes
        FROM subtree
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .fetch_one(&**database)
    .await?;

    Ok(super::types::DeleteImpact {
        deleted_object_count: row.get("deleted_object_count"),
        deleted_file_count: row.get("deleted_file_count"),
        released_storage_bytes: row.get("released_storage_bytes"),
    })
}
