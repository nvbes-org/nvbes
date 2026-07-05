use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::models::{DownloadMetadata, StorageObjectRecord};
use crate::domains::files::models::{StorageObjectStatus, StorageObjectType};
use crate::http::error::AppError;

pub async fn fetch_object_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<StorageObjectRecord, AppError> {
    let object = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        SELECT
          id,
          workspace_id,
          parent_id,
          object_type,
          name,
          size_bytes,
          mime_type,
          status,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at,
          trashed_at
        FROM storage_objects
        WHERE workspace_id = $1
          AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .fetch_optional(&mut **tx)
    .await?;

    object.ok_or_else(|| AppError::not_found("object_not_found", "Object not found."))
}

pub async fn subtree_contains_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
    candidate_id: Uuid,
) -> Result<bool, AppError> {
    let found = sqlx::query_scalar::<_, bool>(
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
        SELECT EXISTS(SELECT 1 FROM subtree WHERE id = $3)
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .bind(candidate_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(found)
}

pub async fn ensure_name_available_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
    exclude_object_id: Option<Uuid>,
) -> Result<(), AppError> {
    let existing = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM storage_objects
        WHERE workspace_id = $1
          AND parent_id IS NOT DISTINCT FROM $2
          AND lower(name) = lower($3)
          AND status IN ('pending', 'active')
          AND ($4::uuid IS NULL OR id <> $4)
        "#,
    )
    .bind(workspace_id)
    .bind(parent_id)
    .bind(name)
    .bind(exclude_object_id)
    .fetch_one(&mut **tx)
    .await?;

    if existing > 0 {
        return Err(AppError::conflict(
            "object_name_conflict",
            "An active object with the same name already exists in this folder.",
        ));
    }

    Ok(())
}

pub async fn list_objects_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
) -> Result<Vec<StorageObjectRecord>, AppError> {
    let objects = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        SELECT
          id,
          workspace_id,
          parent_id,
          object_type,
          name,
          size_bytes,
          mime_type,
          status,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at,
          trashed_at
        FROM storage_objects
        WHERE workspace_id = $1
          AND parent_id IS NOT DISTINCT FROM $2
          AND status IN ('pending', 'active')
        ORDER BY
          CASE object_type
            WHEN 'folder' THEN 0
            ELSE 1
          END,
          lower(name) ASC,
          created_at ASC
        "#,
    )
    .bind(workspace_id)
    .bind(parent_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(objects)
}

pub async fn fetch_download_metadata_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<DownloadMetadata, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          object_type,
          status,
          object_key,
          size_bytes,
          mime_type
        FROM storage_objects
        WHERE workspace_id = $1
          AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .fetch_optional(&mut **tx)
    .await?;

    let row = row.ok_or_else(|| AppError::not_found("object_not_found", "Object not found."))?;

    Ok(DownloadMetadata {
        object_type: row.get("object_type"),
        status: row.get("status"),
        object_key: row.get("object_key"),
        size_bytes: row.get("size_bytes"),
        mime_type: row.get("mime_type"),
    })
}

pub async fn ensure_parent_is_active_folder_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    parent_id: Uuid,
) -> Result<(), AppError> {
    let parent = fetch_object_for_update_tx(tx, workspace_id, parent_id).await?;

    if !matches!(parent.object_type, StorageObjectType::Folder) {
        return Err(AppError::bad_request(
            "invalid_parent",
            "Parent must be a folder.",
        ));
    }

    if !matches!(parent.status, StorageObjectStatus::Active) {
        return Err(AppError::conflict(
            "invalid_parent_state",
            "Parent folder must be active.",
        ));
    }

    Ok(())
}
