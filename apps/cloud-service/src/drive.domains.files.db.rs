use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::models::StorageObjectRecord;
use crate::http::error::AppError;

pub async fn insert_folder_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
    created_by: Uuid,
    created_by_principal_id: Uuid,
) -> Result<StorageObjectRecord, AppError> {
    let folder = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        INSERT INTO storage_objects (
          workspace_id,
          parent_id,
          object_type,
          name,
          size_bytes,
          status,
          created_by,
          created_by_principal_id
        )
        VALUES ($1, $2, 'folder', $3, 0, 'active', $4, $5)
        RETURNING
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
        "#,
    )
    .bind(workspace_id)
    .bind(parent_id)
    .bind(name)
    .bind(created_by)
    .bind(created_by_principal_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(folder)
}

pub async fn update_object_name_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
    name: &str,
) -> Result<StorageObjectRecord, AppError> {
    let updated = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        UPDATE storage_objects
        SET name = $3, updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        RETURNING
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
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .bind(name)
    .fetch_one(&mut **tx)
    .await?;

    Ok(updated)
}

pub async fn update_object_parent_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
    destination_parent_id: Option<Uuid>,
) -> Result<StorageObjectRecord, AppError> {
    let updated = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        UPDATE storage_objects
        SET parent_id = $3, updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        RETURNING
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
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .bind(destination_parent_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(updated)
}
