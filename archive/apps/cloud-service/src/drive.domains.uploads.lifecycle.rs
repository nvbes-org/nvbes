use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::authz::WorkspaceAccess, domains::uploads::models::UploadSessionStatus,
    http::error::AppError,
};

use super::db;
use super::queries;
use super::types::CancelUploadResponse;

#[path = "drive.domains.uploads.lifecycle.complete.rs"]
mod complete;
#[path = "drive.domains.uploads.lifecycle.tus.rs"]
mod tus;

pub use complete::complete_upload;
pub use tus::{append_tus_chunk, get_tus_upload_status};

pub async fn cancel_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    upload_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CancelUploadResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let upload =
        queries::fetch_upload_for_update_tx(&mut tx, access.workspace_id, upload_id).await?;

    if matches!(upload.status, UploadSessionStatus::Completed) {
        return Err(AppError::conflict(
            "upload_already_completed",
            "Completed uploads cannot be cancelled.",
        ));
    }

    if matches!(upload.status, UploadSessionStatus::Cancelled) {
        return Err(AppError::conflict(
            "upload_already_cancelled",
            "Upload is already cancelled.",
        ));
    }

    let cancelled_at = Utc::now();

    if let Some(multipart_upload_id) = upload.storage_multipart_upload_id.as_deref() {
        let storage_object = queries::fetch_storage_object_for_update_tx(
            &mut tx,
            access.workspace_id,
            upload.storage_object_id,
        )
        .await?;
        if let Some(object_key) = storage_object.object_key.as_deref() {
            storage
                .abort_multipart_upload(object_key, multipart_upload_id)
                .await
                .map_err(|e| {
                    AppError::internal(
                        "storage_multipart_abort_failed",
                        format!("Failed to abort multipart upload: {e}"),
                    )
                })?;
        }
    }

    db::cancel_upload_session_tx(&mut tx, access.workspace_id, upload_id, cancelled_at).await?;

    db::delete_pending_storage_object_tx(&mut tx, access.workspace_id, upload.storage_object_id)
        .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "upload.cancelled",
            target_type: "upload_session",
            target_id: Some(upload_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "storage_object_id": upload.storage_object_id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(CancelUploadResponse {
        upload_id,
        storage_object_id: upload.storage_object_id,
        cancelled_at,
    })
}
