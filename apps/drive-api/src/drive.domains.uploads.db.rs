use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::models::StorageObjectRecord;
use super::types::*;
use crate::domains::files::models::{StorageObjectStatus, StorageObjectType};
use crate::http::error::AppError;

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

pub async fn insert_upload_session_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    workspace_id: Uuid,
    storage_object_id: Uuid,
    created_by: Uuid,
    created_by_principal_id: Uuid,
    expected_size_bytes: i64,
    expected_checksum: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO upload_sessions (
          id,
          workspace_id,
          storage_object_id,
          created_by,
          created_by_principal_id,
          expected_size_bytes,
          expected_checksum,
          status,
          expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7)
        "#,
    )
    .bind(id)
    .bind(workspace_id)
    .bind(storage_object_id)
    .bind(created_by)
    .bind(created_by_principal_id)
    .bind(expected_size_bytes)
    .bind(expected_checksum)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn attach_multipart_upload_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    multipart_upload_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET storage_multipart_upload_id = $3,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(multipart_upload_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn insert_upload_part_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    part_number: i32,
    offset_bytes: i64,
    size_bytes: i64,
    etag: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO upload_parts (
          upload_session_id,
          workspace_id,
          part_number,
          offset_bytes,
          size_bytes,
          etag
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(upload_id)
    .bind(workspace_id)
    .bind(part_number)
    .bind(offset_bytes)
    .bind(size_bytes)
    .bind(etag)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn advance_upload_offset_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    upload_offset_bytes: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET upload_offset_bytes = $3,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(upload_offset_bytes)
    .execute(&mut **tx)
    .await?;

    Ok(())
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

pub async fn complete_upload_session_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    completed_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET status = 'completed',
            completed_at = $3,
            updated_at = $3
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(completed_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn cancel_upload_session_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
    cancelled_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE upload_sessions
        SET status = 'cancelled',
            updated_at = $3
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .bind(cancelled_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
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

pub fn storage_object_type_as_str(object_type: StorageObjectType) -> &'static str {
    match object_type {
        StorageObjectType::File => "file",
        StorageObjectType::Folder => "folder",
    }
}

pub fn storage_object_status_as_str(status: StorageObjectStatus) -> &'static str {
    match status {
        StorageObjectStatus::Pending => "pending",
        StorageObjectStatus::Active => "active",
        StorageObjectStatus::Trashed => "trashed",
        StorageObjectStatus::Deleted => "deleted",
        StorageObjectStatus::Quarantined => "quarantined",
    }
}

pub fn map_record_to_view(record: StorageObjectRecord) -> UploadObjectView {
    UploadObjectView {
        id: record.id,
        workspace_id: record.workspace_id,
        parent_id: record.parent_id,
        name: record.name,
        object_type: storage_object_type_as_str(record.object_type).to_owned(),
        status: storage_object_status_as_str(record.status).to_owned(),
        scan_status: record.scan_status,
        size_bytes: record.size_bytes,
        mime_type: record.mime_type,
        checksum: record.checksum,
        created_by: record.created_by,
        created_by_principal_id: record.created_by_principal_id,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}
