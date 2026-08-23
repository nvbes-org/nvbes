use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        files::{
            models::{StorageObjectStatus, StorageObjectType},
            queries as db,
        },
        quotas::BandwidthOutUsageInput,
    },
    http::error::AppError,
};

use crate::domains::files::types::SignedDownloadUrlView;

pub async fn create_download_url(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<super::DownloadUrlResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let metadata = db::fetch_download_metadata_tx(&mut tx, access.workspace_id, object_id).await?;

    if !matches!(metadata.object_type, StorageObjectType::File) {
        return Err(AppError::bad_request(
            "invalid_download_target",
            "Only files can be downloaded.",
        ));
    }

    if matches!(metadata.status, StorageObjectStatus::Quarantined) {
        return Err(AppError::conflict(
            "file_quarantined",
            "This file has been quarantined due to a policy violation.",
        ));
    }

    if !matches!(metadata.status, StorageObjectStatus::Active) {
        return Err(AppError::conflict(
            "object_not_downloadable",
            "Only active files can be downloaded.",
        ));
    }

    let object_key = metadata.object_key.ok_or_else(|| {
        AppError::conflict(
            "missing_object_key",
            "File is missing its storage object key.",
        )
    })?;

    let expires_at = Utc::now() + chrono::Duration::seconds(60);
    let download_url = build_signed_download_url(storage, &object_key, expires_at).await?;

    crate::domains::quotas::record_bandwidth_out_tx(
        &mut tx,
        BandwidthOutUsageInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            storage_object_id: object_id,
            source: "file.download_url_created",
            size_bytes: metadata.size_bytes,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            idempotency_key: format!("file-download-url:{object_id}:{}", Uuid::new_v4()),
        },
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "file.downloaded",
            target_type: "storage_object",
            target_id: Some(object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "size_bytes": metadata.size_bytes,
                "mime_type": metadata.mime_type,
                "download_url_expires_at": expires_at,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(super::DownloadUrlResponse {
        object_id,
        download_url,
    })
}

pub async fn build_signed_download_url(
    storage: &dyn nvbes_storage::ObjectStore,
    object_key: &str,
    expires_at: DateTime<Utc>,
) -> Result<SignedDownloadUrlView, AppError> {
    let expires = (expires_at - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::from_secs(60))
        .min(std::time::Duration::from_secs(60));

    let presigned = storage
        .presign_download(object_key, expires)
        .await
        .map_err(|error| {
            AppError::internal(
                "storage_error",
                format!("Failed to presign download: {error}"),
            )
        })?;

    Ok(SignedDownloadUrlView {
        url: presigned.url,
        method: presigned.method,
        expires_at,
    })
}
