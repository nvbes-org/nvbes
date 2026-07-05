use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::models::{StorageObjectRecord, UploadPartRecord, UploadSessionRecord};
use crate::domains::files::models::StorageObjectStatus;
use crate::domains::files::models::StorageObjectType;
use crate::http::error::AppError;

pub async fn ensure_parent_is_active_folder(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    parent_id: Uuid,
) -> Result<(), AppError> {
    let parent = sqlx::query_as::<_, StorageObjectRecord>(
        r#"
        SELECT
          id,
          workspace_id,
          parent_id,
          object_type,
          name,
          size_bytes,
          mime_type,
          checksum,
          status,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at
        FROM storage_objects
        WHERE workspace_id = $1
          AND id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(parent_id)
    .fetch_optional(pool)
    .await?;

    let parent = parent
        .ok_or_else(|| AppError::not_found("object_not_found", "Parent folder not found."))?;

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

pub async fn fetch_upload_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
) -> Result<UploadSessionRecord, AppError> {
    let upload = sqlx::query_as::<_, UploadSessionRecord>(
        r#"
        SELECT
          storage_object_id,
          expected_size_bytes,
          expected_checksum,
          upload_offset_bytes,
          storage_multipart_upload_id,
          status,
          expires_at
        FROM upload_sessions
        WHERE workspace_id = $1
          AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .fetch_optional(&mut **tx)
    .await?;

    upload.ok_or_else(|| AppError::not_found("upload_not_found", "Upload session not found."))
}

pub async fn list_upload_parts_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    upload_id: Uuid,
) -> Result<Vec<UploadPartRecord>, AppError> {
    let parts = sqlx::query_as::<_, UploadPartRecord>(
        r#"
        SELECT part_number, offset_bytes, size_bytes, etag
        FROM upload_parts
        WHERE workspace_id = $1
          AND upload_session_id = $2
        ORDER BY part_number ASC
        "#,
    )
    .bind(workspace_id)
    .bind(upload_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(parts)
}

pub async fn fetch_storage_object_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    storage_object_id: Uuid,
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
          checksum,
          status,
          scan_status,
          object_key,
          created_by,
          created_by_principal_id,
          created_at,
          updated_at
        FROM storage_objects
        WHERE workspace_id = $1
          AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(storage_object_id)
    .fetch_optional(&mut **tx)
    .await?;

    object.ok_or_else(|| AppError::not_found("object_not_found", "Storage object not found."))
}
