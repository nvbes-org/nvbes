use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::authz::WorkspaceAccess,
    domains::files::models::StorageObjectStatus,
    domains::files::queries::{AuditEventInput, insert_audit_event_tx},
    domains::quotas::FileUploadedUsageInput,
    http::error::AppError,
};

use super::super::types::{AppendTusChunkInput, AppendTusChunkResponse, TusUploadStatusView};
use super::super::{db, logic, queries, scan};

const S3_MIN_NON_FINAL_PART_SIZE_BYTES: i64 = 5 * 1024 * 1024;

pub async fn get_tus_upload_status(
    db: &PgPool,
    access: &WorkspaceAccess,
    upload_id: Uuid,
) -> Result<TusUploadStatusView, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let upload =
        queries::fetch_upload_for_update_tx(&mut tx, access.workspace_id, upload_id).await?;
    logic::ensure_upload_is_appendable(&upload)?;
    tx.commit().await?;

    Ok(TusUploadStatusView {
        upload_id,
        upload_offset_bytes: upload.upload_offset_bytes,
        upload_length_bytes: upload.expected_size_bytes,
        expires_at: upload.expires_at,
    })
}

pub async fn append_tus_chunk(
    storage: &dyn nvbes_storage::ObjectStore,
    scanner: &dyn nvbes_scan::ScanEngine,
    db: &PgPool,
    access: &WorkspaceAccess,
    upload_id: Uuid,
    input: AppendTusChunkInput,
    ip: Option<String>,
    user_agent: Option<String>,
    scan_enabled: bool,
    scan_fail_open: bool,
    scan_engine: &str,
) -> Result<AppendTusChunkResponse, AppError> {
    let requested_offset = logic::validate_tus_offset(input.upload_offset_bytes)?;
    let chunk_size = i64::try_from(input.body.len()).map_err(|e| {
        AppError::bad_request(
            "upload_chunk_size_invalid",
            format!("Upload chunk size is invalid: {e}"),
        )
    })?;

    if chunk_size <= 0 {
        return Err(AppError::bad_request(
            "upload_chunk_empty",
            "TUS PATCH requests must contain at least one byte.",
        ));
    }

    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let upload =
        queries::fetch_upload_for_update_tx(&mut tx, access.workspace_id, upload_id).await?;
    logic::ensure_upload_is_appendable(&upload)?;

    if upload.storage_multipart_upload_id.is_none() {
        return Err(AppError::conflict(
            "upload_not_resumable",
            "This upload session was not created with TUS.",
        ));
    }

    if requested_offset != upload.upload_offset_bytes {
        return Err(AppError::conflict(
            "upload_offset_mismatch",
            "Upload-Offset does not match the server offset.",
        ));
    }

    let next_offset = upload.upload_offset_bytes + chunk_size;
    if next_offset > upload.expected_size_bytes {
        return Err(AppError::bad_request(
            "upload_too_large",
            "Chunk exceeds the declared upload length.",
        ));
    }

    let is_final_chunk = next_offset == upload.expected_size_bytes;
    if !is_final_chunk && chunk_size < S3_MIN_NON_FINAL_PART_SIZE_BYTES {
        return Err(AppError::bad_request(
            "upload_chunk_too_small",
            "Non-final TUS chunks must be at least 5 MiB for S3 multipart uploads.",
        ));
    }

    let storage_object = queries::fetch_storage_object_for_update_tx(
        &mut tx,
        access.workspace_id,
        upload.storage_object_id,
    )
    .await?;

    if !matches!(storage_object.status, StorageObjectStatus::Pending) {
        return Err(AppError::conflict(
            "upload_invalid_object_state",
            "The upload storage object is no longer pending.",
        ));
    }

    let object_key = storage_object.object_key.as_deref().ok_or_else(|| {
        AppError::internal("missing_object_key", "Storage object has no object key")
    })?;
    let multipart_upload_id = upload
        .storage_multipart_upload_id
        .as_deref()
        .ok_or_else(|| {
            AppError::internal("missing_multipart_upload", "Missing multipart upload")
        })?;

    let existing_parts =
        queries::list_upload_parts_tx(&mut tx, access.workspace_id, upload_id).await?;
    let part_number = i32::try_from(existing_parts.len() + 1).map_err(|e| {
        AppError::bad_request(
            "upload_part_number_invalid",
            format!("Upload part number is invalid: {e}"),
        )
    })?;
    let uploaded_part = storage
        .upload_part(object_key, multipart_upload_id, part_number, input.body)
        .await
        .map_err(|e| {
            AppError::internal(
                "storage_upload_part_failed",
                format!("Failed to upload TUS chunk: {e}"),
            )
        })?;

    db::insert_upload_part_tx(
        &mut tx,
        access.workspace_id,
        upload_id,
        uploaded_part.part_number,
        upload.upload_offset_bytes,
        uploaded_part.size_bytes,
        &uploaded_part.etag,
    )
    .await?;
    db::advance_upload_offset_tx(&mut tx, access.workspace_id, upload_id, next_offset).await?;

    if !is_final_chunk {
        tx.commit().await?;
        return Ok(AppendTusChunkResponse {
            upload_offset_bytes: next_offset,
        });
    }

    let parts = queries::list_upload_parts_tx(&mut tx, access.workspace_id, upload_id).await?;
    let mut expected_offset = 0_i64;
    for part in &parts {
        if part.offset_bytes != expected_offset {
            return Err(AppError::conflict(
                "upload_parts_not_contiguous",
                "Stored upload parts are not contiguous.",
            ));
        }
        expected_offset += part.size_bytes;
    }
    if expected_offset != upload.expected_size_bytes {
        return Err(AppError::conflict(
            "upload_parts_size_mismatch",
            "Stored upload parts do not match the declared upload length.",
        ));
    }

    let completed_parts = parts
        .iter()
        .map(|part| nvbes_storage::CompletedUploadPart {
            part_number: part.part_number,
            etag: part.etag.clone(),
        })
        .collect::<Vec<_>>();

    storage
        .complete_multipart_upload(object_key, multipart_upload_id, &completed_parts)
        .await
        .map_err(|e| {
            AppError::internal(
                "storage_multipart_complete_failed",
                format!("Failed to complete multipart upload: {e}"),
            )
        })?;

    let storage_meta = storage.head_object(object_key).await.map_err(|e| {
        AppError::internal(
            "storage_head_failed",
            format!("Failed to read uploaded object metadata: {e}"),
        )
    })?;
    logic::ensure_plain_content_encoding(storage_meta.content_encoding.as_deref())?;

    let file_data = storage.get_object(object_key).await.map_err(|e| {
        AppError::internal(
            "storage_download_failed",
            format!("Failed to download uploaded object: {e}"),
        )
    })?;

    let actual_size = i64::try_from(file_data.len()).map_err(|e| {
        AppError::internal(
            "storage_size_invalid",
            format!("Uploaded object size is invalid: {e}"),
        )
    })?;

    if actual_size != upload.expected_size_bytes {
        return Err(AppError::bad_request(
            "upload_size_mismatch",
            "Uploaded object size does not match the expected size.",
        ));
    }

    let computed_checksum = nvbes_billing::hex_encode(&Sha256::digest(&file_data));
    if let Some(expected_checksum) = &upload.expected_checksum {
        if computed_checksum != *expected_checksum {
            return Err(AppError::bad_request(
                "upload_checksum_mismatch",
                "Uploaded checksum does not match the expected checksum.",
            ));
        }
    }

    crate::domains::quotas::ensure_upload_allowed_tx(&mut tx, access.workspace_id, actual_size)
        .await?;

    let scan_outcome = scan::perform_scan(
        scanner,
        object_key,
        &file_data,
        scan_enabled,
        scan_fail_open,
    )
    .await?;

    let scanned_at = Utc::now();
    let storage_object = if scan_outcome.is_quarantined {
        db::quarantine_storage_object_tx(
            &mut tx,
            access.workspace_id,
            upload.storage_object_id,
            actual_size,
            Some(&computed_checksum),
            scan_engine,
            &scan_outcome.quarantine_reason,
            scanned_at,
        )
        .await?
    } else {
        db::activate_storage_object_tx(
            &mut tx,
            access.workspace_id,
            upload.storage_object_id,
            actual_size,
            Some(&computed_checksum),
            &scan_outcome.status,
            scanned_at,
        )
        .await?
    };

    db::complete_upload_session_tx(&mut tx, access.workspace_id, upload_id, scanned_at).await?;

    crate::domains::quotas::record_file_uploaded_tx(
        &mut tx,
        FileUploadedUsageInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: access.auth.principal_id,
            storage_object_id: upload.storage_object_id,
            upload_id,
            size_bytes: actual_size,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await?;

    let audit_action = if scan_outcome.is_quarantined {
        "file.quarantined"
    } else {
        "file.uploaded"
    };

    insert_audit_event_tx(
        &mut tx,
        AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: audit_action,
            target_type: "storage_object",
            target_id: Some(upload.storage_object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "upload_id": upload_id,
                "size_bytes": actual_size,
                "checksum": computed_checksum,
                "scan_status": scan_outcome.status,
                "quarantine_reason": if scan_outcome.is_quarantined { Some(&scan_outcome.quarantine_reason) } else { None },
                "tus": true,
            }),
        },
    )
    .await?;

    let _ = storage_object;
    tx.commit().await?;

    Ok(AppendTusChunkResponse {
        upload_offset_bytes: next_offset,
    })
}
