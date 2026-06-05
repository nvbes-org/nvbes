use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

use crate::db::Database;

const STORAGE_DELETE_BATCH_SIZE: usize = 1000;

#[derive(Debug, FromRow)]
struct StoragePurgeCandidate {
    id: Uuid,
    object_key: Option<String>,
}

pub async fn purge_quarantined(
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
    retention_days: u32,
) -> anyhow::Result<JsonValue> {
    let candidates = sqlx::query_as::<_, StoragePurgeCandidate>(
        r#"
        SELECT id, object_key
        FROM storage_objects
        WHERE status = 'quarantined'
          AND updated_at < NOW() - ($1::integer * INTERVAL '1 day')
        ORDER BY updated_at ASC
        LIMIT 1000
        "#,
    )
    .bind(retention_days as i32)
    .fetch_all(&**database)
    .await?;

    let result = purge_candidates(database, storage, "quarantined", candidates).await?;

    Ok(serde_json::json!({
        "deleted_objects": result.deleted_metadata,
        "storage_objects": result.deleted_storage_objects
    }))
}

pub async fn purge_deleted_storage(
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<JsonValue> {
    let candidates = sqlx::query_as::<_, StoragePurgeCandidate>(
        r#"
        SELECT id, object_key
        FROM storage_objects
        WHERE status = 'deleted'
        ORDER BY updated_at ASC
        LIMIT 1000
        "#,
    )
    .fetch_all(&**database)
    .await?;

    let result = purge_candidates(database, storage, "deleted", candidates).await?;

    Ok(serde_json::json!({
        "deleted_objects": result.deleted_metadata,
        "storage_objects": result.deleted_storage_objects
    }))
}

pub async fn purge_workspace_deleted_storage(
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
    workspace_id: Uuid,
) -> anyhow::Result<PurgeResult> {
    let candidates = sqlx::query_as::<_, StoragePurgeCandidate>(
        r#"
        SELECT id, object_key
        FROM storage_objects
        WHERE workspace_id = $1
          AND status = 'deleted'
        ORDER BY updated_at ASC
        LIMIT 1000
        "#,
    )
    .bind(workspace_id)
    .fetch_all(&**database)
    .await?;

    purge_candidates(database, storage, "deleted", candidates).await
}

pub struct PurgeResult {
    pub deleted_metadata: u64,
    pub deleted_storage_objects: usize,
}

async fn purge_candidates(
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
    expected_status: &str,
    candidates: Vec<StoragePurgeCandidate>,
) -> anyhow::Result<PurgeResult> {
    if candidates.is_empty() {
        return Ok(PurgeResult {
            deleted_metadata: 0,
            deleted_storage_objects: 0,
        });
    }

    let object_keys = candidates
        .iter()
        .filter_map(|candidate| candidate.object_key.clone())
        .collect::<Vec<_>>();

    for keys in object_keys.chunks(STORAGE_DELETE_BATCH_SIZE) {
        storage.delete_objects(keys).await?;
    }

    let ids = candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();

    let result = sqlx::query(
        r#"
        DELETE FROM storage_objects
        WHERE id = ANY($1)
          AND status = $2::storage_object_status
        "#,
    )
    .bind(&ids)
    .bind(expected_status)
    .execute(&**database)
    .await?;

    Ok(PurgeResult {
        deleted_metadata: result.rows_affected(),
        deleted_storage_objects: object_keys.len(),
    })
}
