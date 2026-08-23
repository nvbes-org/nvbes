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
        SELECT so.id, so.object_key
        FROM storage_objects so
        INNER JOIN data_retention_policies retention
          ON retention.category = so.retention_category
        WHERE so.status = 'quarantined'
          AND COALESCE(
            so.retention_until,
            so.updated_at
              + (LEAST($1::integer, retention.deleted_retention_days) * INTERVAL '1 day')
          ) <= NOW()
        ORDER BY so.updated_at ASC
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
        SELECT so.id, so.object_key
        FROM storage_objects so
        INNER JOIN data_retention_policies retention
          ON retention.category = so.retention_category
        WHERE so.status = 'deleted'
          AND COALESCE(
            so.retention_until,
            so.updated_at + (retention.deleted_retention_days * INTERVAL '1 day')
          ) <= NOW()
        ORDER BY so.updated_at ASC
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
        SELECT so.id, so.object_key
        FROM storage_objects so
        INNER JOIN data_retention_policies retention
          ON retention.category = so.retention_category
        WHERE so.workspace_id = $1
          AND so.status = 'deleted'
          AND COALESCE(
            so.retention_until,
            so.updated_at + (retention.deleted_retention_days * INTERVAL '1 day')
          ) <= NOW()
        ORDER BY so.updated_at ASC
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

    let ids = candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();
    cryptographically_erase_candidates(database, &ids).await?;

    for keys in object_keys.chunks(STORAGE_DELETE_BATCH_SIZE) {
        storage.delete_objects(keys).await?;
    }

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

async fn cryptographically_erase_candidates(
    database: &Database,
    storage_object_ids: &[Uuid],
) -> anyhow::Result<()> {
    let mut tx = database.begin().await?;
    sqlx::query(
        r#"
        DELETE FROM storage_object_key_envelopes
        WHERE storage_object_id = ANY($1)
          AND destroyed_at IS NULL
        "#,
    )
    .bind(storage_object_ids)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE storage_objects
        SET cryptographic_erased_at = COALESCE(cryptographic_erased_at, NOW()),
            updated_at = NOW()
        WHERE id = ANY($1)
        "#,
    )
    .bind(storage_object_ids)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
