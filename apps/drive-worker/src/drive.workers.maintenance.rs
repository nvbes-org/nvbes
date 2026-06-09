use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

use crate::db::Database;

#[path = "drive.workers.maintenance.storage.rs"]
pub mod storage;

pub const JOB_UPLOADS_PURGE_EXPIRED: &str = "uploads.purge_expired";
pub const JOB_QUOTAS_RECALCULATE: &str = "quotas.recalculate";
pub const JOB_TRASH_PURGE: &str = "trash.purge";
pub const JOB_STORAGE_PURGE_DELETED: &str = "storage.purge_deleted";
pub const JOB_STORAGE_PURGE_QUARANTINED: &str = "storage.purge_quarantined";

pub async fn enqueue_maintenance_jobs(redis: &nvbes_redis::RedisPool) -> anyhow::Result<()> {
    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_UPLOADS_PURGE_EXPIRED.to_string(),
            job_type: JOB_UPLOADS_PURGE_EXPIRED.to_string(),
            payload: serde_json::json!({}),
            idempotency_key: Some("uploads.purge_expired.daily".to_string()),
            max_attempts: 3,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await?;

    nvbes_redis::worker_queue::enqueue_job(
        redis,
        nvbes_redis::worker_queue::EnqueueJobInput {
            queue: JOB_STORAGE_PURGE_DELETED.to_string(),
            job_type: JOB_STORAGE_PURGE_DELETED.to_string(),
            payload: serde_json::json!({}),
            idempotency_key: Some("storage.purge_deleted.daily".to_string()),
            max_attempts: 3,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await?;

    Ok(())
}

#[derive(Debug, FromRow)]
struct ExpiredUploadCandidate {
    upload_id: Uuid,
    workspace_id: Uuid,
    storage_object_id: Uuid,
    object_key: Option<String>,
    storage_multipart_upload_id: Option<String>,
}

pub async fn purge_expired_uploads(
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<JsonValue> {
    let mut tx = database.begin().await?;
    let candidates = sqlx::query_as::<_, ExpiredUploadCandidate>(
        r#"
        SELECT
          us.id AS upload_id,
          us.workspace_id,
          us.storage_object_id,
          so.object_key,
          us.storage_multipart_upload_id
        FROM upload_sessions us
        INNER JOIN storage_objects so
          ON so.workspace_id = us.workspace_id
         AND so.id = us.storage_object_id
        WHERE us.expires_at < NOW()
          AND us.status IN ('pending', 'expired')
          AND so.status = 'pending'
        ORDER BY us.expires_at ASC
        LIMIT 100
        FOR UPDATE OF us, so SKIP LOCKED
        "#,
    )
    .fetch_all(&mut *tx)
    .await?;

    for candidate in &candidates {
        sqlx::query(
            r#"
            UPDATE upload_sessions
            SET status = 'expired',
                updated_at = NOW()
            WHERE workspace_id = $1
              AND id = $2
              AND status IN ('pending', 'expired')
            "#,
        )
        .bind(candidate.workspace_id)
        .bind(candidate.upload_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let mut purged_storage_objects = Vec::new();
    let mut storage_failures = 0_u64;

    for candidate in candidates {
        let Some(object_key) = candidate.object_key.as_deref() else {
            purged_storage_objects.push(candidate.storage_object_id);
            continue;
        };

        if let Some(multipart_upload_id) = candidate.storage_multipart_upload_id.as_deref()
            && let Err(err) = storage
                .abort_multipart_upload(object_key, multipart_upload_id)
                .await
        {
            tracing::warn!(
                upload_id = %candidate.upload_id,
                storage_object_id = %candidate.storage_object_id,
                error = %err,
                "failed to abort expired multipart upload before object cleanup"
            );
        }

        match storage.delete_objects(&[object_key.to_owned()]).await {
            Ok(()) => purged_storage_objects.push(candidate.storage_object_id),
            Err(err) => {
                storage_failures += 1;
                tracing::warn!(
                    upload_id = %candidate.upload_id,
                    storage_object_id = %candidate.storage_object_id,
                    error = %err,
                    "failed to purge expired upload storage"
                );
            }
        }
    }

    let mut deleted_metadata = 0_u64;
    for storage_object_id in &purged_storage_objects {
        let result = sqlx::query(
            r#"
            DELETE FROM storage_objects
            WHERE id = $1
              AND status = 'pending'
            "#,
        )
        .bind(storage_object_id)
        .execute(&**database)
        .await?;
        deleted_metadata += result.rows_affected();
    }

    Ok(serde_json::json!({
        "claimed_expired_uploads": purged_storage_objects.len() + storage_failures as usize,
        "deleted_pending_objects": deleted_metadata,
        "storage_failures": storage_failures
    }))
}

pub async fn recalculate_quotas(database: &Database) -> anyhow::Result<JsonValue> {
    let result = sqlx::query(
        r#"
        UPDATE quota_usage qu
        SET storage_used_bytes = COALESCE((
            SELECT SUM(so.size_bytes)
            FROM storage_objects so
            WHERE so.workspace_id = qu.workspace_id
              AND so.status = 'active'
              AND so.object_type = 'file'
        ), 0),
        updated_at = NOW()
        "#,
    )
    .execute(&**database)
    .await?;

    Ok(serde_json::json!({
        "updated_workspaces": result.rows_affected()
    }))
}

pub async fn purge_trash(database: &Database) -> anyhow::Result<JsonValue> {
    let result = sqlx::query(
        r#"
        UPDATE storage_objects
        SET status = 'deleted',
            updated_at = NOW()
        WHERE status = 'trashed'
          AND trashed_at < NOW() - INTERVAL '30 days'
        "#,
    )
    .execute(&**database)
    .await?;

    Ok(serde_json::json!({
        "deleted_objects": result.rows_affected()
    }))
}
