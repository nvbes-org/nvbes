use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::models::StorageObjectRecord;
use crate::http::error::AppError;

pub struct DeleteImpact {
    pub deleted_object_count: i64,
    pub deleted_file_count: i64,
    pub released_storage_bytes: i64,
}

pub async fn list_trash_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
) -> Result<Vec<StorageObjectRecord>, AppError> {
    let objects = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        SELECT
          so.id,
          so.workspace_id,
          so.parent_id,
          so.object_type,
          so.name,
          so.size_bytes,
          so.mime_type,
          so.status,
          so.created_by,
          so.created_by_principal_id,
          so.created_at,
          so.updated_at,
          so.trashed_at
        FROM storage_objects so
        LEFT JOIN storage_objects parent ON parent.id = so.parent_id
        WHERE so.workspace_id = $1
          AND so.status = 'trashed'
          AND (parent.id IS NULL OR parent.status <> 'trashed')
        ORDER BY so.trashed_at DESC NULLS LAST, lower(so.name) ASC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(objects)
}

pub async fn trash_subtree_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<Vec<StorageObjectRecord>, AppError> {
    let records = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        WITH RECURSIVE subtree AS (
          SELECT id
          FROM storage_objects
          WHERE workspace_id = $1 AND id = $2
          UNION ALL
          SELECT child.id
          FROM storage_objects child
          INNER JOIN subtree parent_tree ON child.parent_id = parent_tree.id
          WHERE child.workspace_id = $1
        )
        UPDATE storage_objects so
        SET status = 'trashed',
            trashed_at = NOW(),
            updated_at = NOW()
        FROM subtree
        WHERE so.id = subtree.id
        RETURNING
          so.id,
          so.workspace_id,
          so.parent_id,
          so.object_type,
          so.name,
          so.size_bytes,
          so.mime_type,
          so.status,
          so.created_by,
          so.created_by_principal_id,
          so.created_at,
          so.updated_at,
          so.trashed_at
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(records)
}

pub async fn restore_subtree_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<Vec<StorageObjectRecord>, AppError> {
    let records = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        WITH RECURSIVE subtree AS (
          SELECT id
          FROM storage_objects
          WHERE workspace_id = $1 AND id = $2
          UNION ALL
          SELECT child.id
          FROM storage_objects child
          INNER JOIN subtree parent_tree ON child.parent_id = parent_tree.id
          WHERE child.workspace_id = $1
        )
        UPDATE storage_objects so
        SET status = 'active',
            trashed_at = NULL,
            updated_at = NOW()
        FROM subtree
        WHERE so.id = subtree.id
        RETURNING
          so.id,
          so.workspace_id,
          so.parent_id,
          so.object_type,
          so.name,
          so.size_bytes,
          so.mime_type,
          so.status,
          so.created_by,
          so.created_by_principal_id,
          so.created_at,
          so.updated_at,
          so.trashed_at
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(records)
}

pub async fn mark_subtree_deleted_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        WITH RECURSIVE subtree AS (
          SELECT id
          FROM storage_objects
          WHERE workspace_id = $1 AND id = $2
          UNION ALL
          SELECT child.id
          FROM storage_objects child
          INNER JOIN subtree parent_tree ON child.parent_id = parent_tree.id
          WHERE child.workspace_id = $1
        )
        UPDATE storage_objects so
        SET status = 'deleted',
            updated_at = NOW()
        FROM subtree
        WHERE so.id = subtree.id
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn compute_delete_impact_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<DeleteImpact, AppError> {
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
    .fetch_one(&mut **tx)
    .await?;

    use sqlx::Row;
    Ok(DeleteImpact {
        deleted_object_count: row.get("deleted_object_count"),
        deleted_file_count: row.get("deleted_file_count"),
        released_storage_bytes: row.get("released_storage_bytes"),
    })
}
