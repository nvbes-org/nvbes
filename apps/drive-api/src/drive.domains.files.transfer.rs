use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        files::models::{StorageObjectStatus, StorageObjectType},
        quotas::BandwidthOutUsageInput,
    },
    http::error::AppError,
};
use sqlx::PgPool;

use super::queries as db;
use super::queries::AuditEventInput;
pub use super::types::*;

enum ResolvedRange {
    Full,
    Partial { start: i64, end_inclusive: i64 },
    Unsatisfiable,
}

pub async fn create_download_url(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<DownloadUrlResponse, AppError> {
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

    let expires_at = Utc::now() + chrono::Duration::minutes(5);
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

    db::insert_audit_event_tx(
        &mut tx,
        AuditEventInput {
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

    Ok(DownloadUrlResponse {
        object_id,
        download_url,
    })
}

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
            let body = storage.get_object(&object_key).await.map_err(|e| {
                AppError::internal(
                    "storage_download_failed",
                    format!("Failed to download object: {e}"),
                )
            })?;
            let served_bytes = i64::try_from(body.len()).map_err(|e| {
                AppError::internal(
                    "storage_size_invalid",
                    format!("Downloaded object size is invalid: {e}"),
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
                .map_err(|e| {
                    AppError::internal(
                        "storage_download_failed",
                        format!("Failed to download object range: {e}"),
                    )
                })?;
            let served_bytes = i64::try_from(body.len()).map_err(|e| {
                AppError::internal(
                    "storage_size_invalid",
                    format!("Downloaded range size is invalid: {e}"),
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

    db::insert_audit_event_tx(
        &mut tx,
        AuditEventInput {
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

pub async fn build_signed_download_url(
    storage: &dyn nvbes_storage::ObjectStore,
    object_key: &str,
    expires_at: DateTime<Utc>,
) -> Result<SignedDownloadUrlView, AppError> {
    let expires = (expires_at - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::from_secs(300));

    let presigned = storage
        .presign_download(object_key, expires)
        .await
        .map_err(|e| {
            AppError::internal("storage_error", format!("Failed to presign download: {e}"))
        })?;

    Ok(SignedDownloadUrlView {
        url: presigned.url,
        method: presigned.method,
        expires_at,
    })
}

fn ensure_downloadable_file(
    metadata: &crate::domains::files::models::DownloadMetadata,
) -> Result<(), AppError> {
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

    Ok(())
}

fn resolve_range(range_header: Option<&str>, size_bytes: i64) -> Result<ResolvedRange, AppError> {
    let Some(range_header) = range_header else {
        return Ok(ResolvedRange::Full);
    };

    let range_header = range_header.trim();
    let Some(spec) = range_header.strip_prefix("bytes=") else {
        return Err(AppError::bad_request(
            "invalid_range_header",
            "Range must use the bytes unit.",
        ));
    };

    if spec.contains(',') {
        return Err(AppError::bad_request(
            "multiple_ranges_unsupported",
            "Multiple byte ranges are not supported.",
        ));
    }

    if size_bytes <= 0 {
        return Ok(ResolvedRange::Unsatisfiable);
    }

    let Some((start_raw, end_raw)) = spec.split_once('-') else {
        return Err(AppError::bad_request(
            "invalid_range_header",
            "Range must use start-end syntax.",
        ));
    };

    if start_raw.is_empty() {
        let suffix_len = parse_non_negative_i64(end_raw, "range_suffix")?;
        if suffix_len == 0 {
            return Ok(ResolvedRange::Unsatisfiable);
        }
        let start = (size_bytes - suffix_len).max(0);
        return Ok(ResolvedRange::Partial {
            start,
            end_inclusive: size_bytes - 1,
        });
    }

    let start = parse_non_negative_i64(start_raw, "range_start")?;
    if start >= size_bytes {
        return Ok(ResolvedRange::Unsatisfiable);
    }

    let end_inclusive = if end_raw.is_empty() {
        size_bytes - 1
    } else {
        parse_non_negative_i64(end_raw, "range_end")?.min(size_bytes - 1)
    };

    if end_inclusive < start {
        return Ok(ResolvedRange::Unsatisfiable);
    }

    Ok(ResolvedRange::Partial {
        start,
        end_inclusive,
    })
}

fn parse_non_negative_i64(value: &str, code: &str) -> Result<i64, AppError> {
    let parsed = value.parse::<i64>().map_err(|e| {
        AppError::bad_request(code, format!("Range value must be a valid integer: {e}"))
    })?;
    if parsed < 0 {
        return Err(AppError::bad_request(
            code,
            "Range value cannot be negative.",
        ));
    }
    Ok(parsed)
}
