use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        files::{
            queries as db,
            types::{DownloadObjectInput, DownloadObjectResponse, DownloadObjectStatus},
        },
        quotas::BandwidthOutUsageInput,
    },
    http::error::AppError,
};

use super::ResolvedRange;
use super::range::{ensure_downloadable_file, resolve_range};

pub async fn download_object(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: DownloadObjectInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<DownloadObjectResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let metadata = db::fetch_download_metadata_tx(&mut tx, access.workspace_id, object_id).await?;
    ensure_downloadable_file(&metadata)?;

    let object_key = metadata.object_key.clone().ok_or_else(|| {
        AppError::conflict(
            "missing_object_key",
            "File is missing its storage object key.",
        )
    })?;
    let resolved_range = resolve_range(input.range_header.as_deref(), metadata.size_bytes)?;

    let (body, status, served_bytes) = match resolved_range {
        ResolvedRange::Full => {
            let body = storage.get_object(&object_key).await.map_err(|error| {
                AppError::internal(
                    "storage_download_failed",
                    format!("Failed to download object: {error}"),
                )
            })?;
            let served_bytes = i64::try_from(body.len()).map_err(|error| {
                AppError::internal(
                    "storage_size_invalid",
                    format!("Downloaded object size is invalid: {error}"),
                )
            })?;
            (body, DownloadObjectStatus::Full, served_bytes)
        }
        ResolvedRange::Partial {
            start,
            end_inclusive,
        } => {
            let body = storage
                .get_object_range(
                    &object_key,
                    nvbes_storage::ByteRange {
                        start,
                        end_inclusive,
                    },
                )
                .await
                .map_err(|error| {
                    AppError::internal(
                        "storage_download_failed",
                        format!("Failed to download object range: {error}"),
                    )
                })?;
            let served_bytes = i64::try_from(body.len()).map_err(|error| {
                AppError::internal(
                    "storage_size_invalid",
                    format!("Downloaded range size is invalid: {error}"),
                )
            })?;
            (
                body,
                DownloadObjectStatus::Partial {
                    start,
                    end_inclusive,
                },
                served_bytes,
            )
        }
        ResolvedRange::Unsatisfiable => {
            tx.commit().await?;
            return Ok(DownloadObjectResponse {
                body: Vec::new(),
                status: DownloadObjectStatus::Unsatisfiable,
                size_bytes: metadata.size_bytes,
                content_type: metadata
                    .mime_type
                    .unwrap_or_else(|| "application/octet-stream".to_owned()),
                served_bytes: 0,
            });
        }
    };

    crate::domains::quotas::record_bandwidth_out_tx(
        &mut tx,
        BandwidthOutUsageInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            storage_object_id: object_id,
            source: "file.download_streamed",
            size_bytes: served_bytes,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            idempotency_key: format!("file-download-stream:{object_id}:{}", Uuid::new_v4()),
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
                "served_bytes": served_bytes,
                "mime_type": metadata.mime_type,
                "range": input.range_header,
                "streamed": true,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(DownloadObjectResponse {
        body,
        status,
        size_bytes: metadata.size_bytes,
        content_type: metadata
            .mime_type
            .unwrap_or_else(|| "application/octet-stream".to_owned()),
        served_bytes,
    })
}
