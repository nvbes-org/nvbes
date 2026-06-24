use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::db;
use super::logic::{
    billing_status_blocks_upload, storage_alerts, storage_usage_percent, upload_blocked,
};
use super::observability;
pub use super::types::{
    BandwidthOutUsageInput, FileUploadedUsageInput, QuotaResponse, StorageReleasedUsageInput,
    StorageThresholdAuditInput,
};
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

pub async fn get_quota(db: &PgPool, access: &WorkspaceAccess) -> Result<QuotaResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    db::ensure_quota_row(&mut tx, access.workspace_id).await?;

    let row = db::fetch_quota_row(&mut tx, access.workspace_id).await?;

    tx.commit().await?;

    let row =
        row.ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;
    let included_storage_bytes: i64 = row.get("included_storage_bytes");
    let used_storage_bytes: i64 = row.get("used_storage_bytes");

    Ok(QuotaResponse {
        workspace_id: row.get("workspace_id"),
        used_storage_bytes,
        included_storage_bytes,
        file_count: row.get("file_count"),
        bandwidth_out_bytes_month: row.get("bandwidth_out_bytes_month"),
        storage_usage_percent: storage_usage_percent(used_storage_bytes, included_storage_bytes),
        upload_blocked: upload_blocked(used_storage_bytes, included_storage_bytes),
        alerts: storage_alerts(used_storage_bytes, included_storage_bytes),
        updated_at: row.get("updated_at"),
    })
}

pub async fn ensure_upload_allowed_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    requested_size_bytes: i64,
) -> Result<(), AppError> {
    if let Some(status) = db::lock_subscription_status(tx, workspace_id).await?
        && billing_status_blocks_upload(&status)
    {
        return Err(AppError::forbidden(
            "billing_access_suspended",
            "Workspace billing policy has suspended new uploads.",
        ));
    }

    let snapshot = db::lock_usage_snapshot(tx, workspace_id).await?;

    if !snapshot.upload_allowed {
        return Err(AppError::forbidden(
            "billing_upload_not_entitled",
            "Workspace entitlements do not allow new uploads.",
        ));
    }

    if upload_blocked(snapshot.used_storage_bytes, snapshot.included_storage_bytes) {
        return Err(AppError::conflict(
            "quota_critical_exceeded",
            "Workspace storage quota is already at or above 100%; new uploads are blocked.",
        ));
    }

    if snapshot
        .used_storage_bytes
        .saturating_add(requested_size_bytes)
        > snapshot.included_storage_bytes
    {
        return Err(AppError::conflict(
            "quota_exceeded",
            "Workspace storage quota would be exceeded by this upload.",
        ));
    }

    Ok(())
}

pub async fn record_file_uploaded_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: FileUploadedUsageInput<'_>,
) -> Result<(), AppError> {
    let before = db::lock_usage_snapshot(tx, input.workspace_id).await?;

    db::increment_storage_usage(tx, input.workspace_id, input.size_bytes).await?;

    db::insert_usage_event(
        tx,
        input.workspace_id,
        "storage_bytes",
        input.size_bytes,
        "bytes",
        "upload.complete",
        Some(format!("upload:{}:storage-bytes", input.upload_id)),
    )
    .await?;
    db::insert_usage_event(
        tx,
        input.workspace_id,
        "file_count",
        1,
        "file",
        "upload.complete",
        Some(format!("upload:{}:file-count", input.upload_id)),
    )
    .await?;

    let after_used = before.used_storage_bytes.saturating_add(input.size_bytes);
    observability::insert_storage_threshold_audits(
        tx,
        StorageThresholdAuditInput {
            workspace_id: input.workspace_id,
            before: &before,
            after_used_storage_bytes: after_used,
            actor_user_id: input.actor_user_id,
            actor_principal_id: input.actor_principal_id,
            storage_object_id: input.storage_object_id,
            ip: input.ip,
            user_agent: input.user_agent,
        },
    )
    .await?;

    Ok(())
}

pub async fn release_storage_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: StorageReleasedUsageInput<'_>,
) -> Result<(), AppError> {
    db::ensure_quota_row(tx, input.workspace_id).await?;

    db::decrement_storage_usage(
        tx,
        input.workspace_id,
        input.released_storage_bytes,
        input.deleted_file_count,
    )
    .await?;

    if input.released_storage_bytes > 0 {
        db::insert_usage_event(
            tx,
            input.workspace_id,
            "storage_bytes",
            -input.released_storage_bytes,
            "bytes",
            input.source,
            Some(format!(
                "{}:{}:storage-release",
                input.source, input.target_id
            )),
        )
        .await?;
    }

    if input.deleted_file_count > 0 {
        db::insert_usage_event(
            tx,
            input.workspace_id,
            "file_count",
            -input.deleted_file_count,
            "file",
            input.source,
            Some(format!("{}:{}:file-release", input.source, input.target_id)),
        )
        .await?;
    }

    crate::domains::audit::record_event_tx(
        tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: input.workspace_id,
            actor_user_id: input.actor_user_id,
            actor_principal_id: input.actor_principal_id,
            action: "quota.storage_released",
            target_type: "storage_object",
            target_id: Some(input.target_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "source": input.source,
                "released_storage_bytes": input.released_storage_bytes,
                "deleted_file_count": input.deleted_file_count,
            }),
        },
    )
    .await?;

    Ok(())
}

pub async fn record_bandwidth_out_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: BandwidthOutUsageInput<'_>,
) -> Result<(), AppError> {
    db::ensure_quota_row(tx, input.workspace_id).await?;

    db::insert_usage_event(
        tx,
        input.workspace_id,
        "bandwidth_out_bytes",
        input.size_bytes,
        "bytes",
        input.source,
        Some(input.idempotency_key),
    )
    .await?;

    db::update_bandwidth_usage_cache(tx, input.workspace_id).await?;

    crate::domains::audit::record_event_tx(
        tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: input.workspace_id,
            actor_user_id: input.actor_user_id,
            actor_principal_id: input.actor_principal_id,
            action: "quota.bandwidth_out_recorded",
            target_type: "storage_object",
            target_id: Some(input.storage_object_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "source": input.source,
                "size_bytes": input.size_bytes,
            }),
        },
    )
    .await?;

    Ok(())
}
