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

use super::super::types::{CompleteUploadInput, CompleteUploadResponse};
use super::super::{db, logic, queries, scan};

pub async fn complete_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    scanner: &dyn nvbes_scan::ScanEngine,
    db: &PgPool,
    access: &WorkspaceAccess,
    upload_id: Uuid,
    input: CompleteUploadInput,
    ip: Option<String>,
    user_agent: Option<String>,
    scan_enabled: bool,
    scan_fail_open: bool,
    scan_engine: &str,
) -> Result<CompleteUploadResponse, AppError> {
    let size_bytes = logic::validate_size(input.size_bytes)?;
    let checksum = logic::normalize_checksum(input.checksum)?;
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let upload =
        queries::fetch_upload_for_update_tx(&mut tx, access.workspace_id, upload_id).await?;
    logic::ensure_upload_is_completable(&upload)?;

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

    if size_bytes != upload.expected_size_bytes {
        return Err(AppError::bad_request(
            "upload_size_mismatch",
            "Uploaded size does not match the expected size.",
        ));
    }

    let object_key = storage_object.object_key.as_deref().ok_or_else(|| {
        AppError::internal("missing_object_key", "Storage object has no object key")
    })?;

    let storage_meta = storage.head_object(object_key).await.map_err(|e| {
        AppError::internal(
            "storage_head_failed",
            &format!("Failed to read uploaded object metadata: {e}"),
        )
    })?;

    if storage_meta.size_bytes != upload.expected_size_bytes
        || storage_meta.size_bytes != size_bytes
    {
        return Err(AppError::bad_request(
            "upload_size_mismatch",
            "Uploaded object size does not match the expected size.",
        ));
    }
    logic::ensure_plain_content_encoding(storage_meta.content_encoding.as_deref())?;

    let file_data = storage.get_object(object_key).await.map_err(|e| {
        AppError::internal(
            "storage_download_failed",
            &format!("Failed to download uploaded object: {e}"),
        )
    })?;

    let actual_size = i64::try_from(file_data.len()).map_err(|e| {
        AppError::internal(
            "storage_size_invalid",
            &format!("Uploaded object size is invalid: {e}"),
        )
    })?;
    if actual_size != storage_meta.size_bytes {
        return Err(AppError::bad_request(
            "upload_size_mismatch",
            "Downloaded object size does not match storage metadata.",
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

    if let Some(request_checksum) = checksum.as_deref() {
        if request_checksum != computed_checksum {
            return Err(AppError::bad_request(
                "upload_checksum_mismatch",
                "Uploaded checksum does not match the stored object.",
            ));
        }
    }

    crate::domains::quotas::ensure_upload_allowed_tx(&mut tx, access.workspace_id, actual_size)
        .await?;

    let scan_outcome =
        scan::perform_scan(scanner, &file_data, scan_enabled, scan_fail_open).await?;

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
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(CompleteUploadResponse {
        upload_id,
        storage_object: db::map_record_to_view(storage_object),
        activated_at: scanned_at,
    })
}
