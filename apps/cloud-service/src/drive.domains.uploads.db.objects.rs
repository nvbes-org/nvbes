use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{domains::uploads::models::StorageObjectRecord, http::error::AppError};

#[expect(
    clippy::too_many_arguments,
    reason = "Pending object creation persists explicit upload ownership and storage metadata."
)]
pub async fn insert_pending_storage_object_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
    mime_type: &str,
    object_key: &str,
    created_by: Uuid,
    created_by_principal_id: Uuid,
) -> Result<StorageObjectRecord, AppError> {
    let storage_object = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        INSERT INTO storage_objects (
          id,
          workspace_id,
          parent_id,
          object_type,
          name,
          size_bytes,
          mime_type,
          object_key,
          status,
          created_by,
          created_by_principal_id
        )
        VALUES ($1, $2, $3, 'file', $4, 0, $5, $6, 'pending', $7, $8)
        RETURNING
          id,
          workspace_id,
          parent_id,
          object_type,
          name,
          size_bytes,
          mime_type,
          checksum,
          status,
          scan_status,
          object_key,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at
        "#,
    )
    .bind(id)
    .bind(workspace_id)
    .bind(parent_id)
    .bind(name)
    .bind(mime_type)
    .bind(object_key)
    .bind(created_by)
    .bind(created_by_principal_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(storage_object)
}

pub async fn activate_storage_object_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    storage_object_id: Uuid,
    size_bytes: i64,
    checksum: Option<&str>,
    scan_status: &str,
    activated_at: DateTime<Utc>,
) -> Result<StorageObjectRecord, AppError> {
    let storage_object = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        UPDATE storage_objects
        SET size_bytes = $3,
            checksum = $4,
            status = 'active',
            scan_status = $5,
            scanned_at = $6,
            updated_at = $6
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
          checksum,
          status,
          scan_status,
          object_key,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(storage_object_id)
    .bind(size_bytes)
    .bind(checksum)
    .bind(scan_status)
    .bind(activated_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(storage_object)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Quarantine persistence keeps scan outcome fields explicit."
)]
pub async fn quarantine_storage_object_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    storage_object_id: Uuid,
    size_bytes: i64,
    checksum: Option<&str>,
    scan_engine: &str,
    quarantine_reason: &str,
    scanned_at: DateTime<Utc>,
) -> Result<StorageObjectRecord, AppError> {
    let storage_object = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        UPDATE storage_objects
        SET size_bytes = $3,
            checksum = $4,
            status = 'quarantined',
            scan_status = 'infected',
            scanned_at = $5,
            scan_engine = $6,
            quarantine_reason = $7,
            updated_at = $5
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
          checksum,
          status,
          scan_status,
          object_key,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(storage_object_id)
    .bind(size_bytes)
    .bind(checksum)
    .bind(scanned_at)
    .bind(scan_engine)
    .bind(quarantine_reason)
    .fetch_one(&mut **tx)
    .await?;

    Ok(storage_object)
}

pub async fn delete_pending_storage_object_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    storage_object_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM storage_objects
        WHERE workspace_id = $1
          AND id = $2
          AND status = 'pending'
        "#,
    )
    .bind(workspace_id)
    .bind(storage_object_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
