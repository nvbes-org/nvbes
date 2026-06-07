use chrono::Utc;
use sqlx::Postgres;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        files::queries::{AuditEventInput, insert_audit_event_tx},
        quotas::FileUploadedUsageInput,
        uploads::{db, scan},
    },
    http::error::AppError,
};

pub(super) struct TusFinalizeInput<'a> {
    pub scanner: &'a dyn nvbes_scan::ScanEngine,
    pub storage_object_id: Uuid,
    pub upload_id: Uuid,
    pub file_data: &'a [u8],
    pub actual_size: i64,
    pub computed_checksum: &'a str,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub scan_enabled: bool,
    pub scan_fail_open: bool,
    pub scan_engine: &'a str,
}

pub(super) async fn finalize_tus_upload(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    access: &WorkspaceAccess,
    input: TusFinalizeInput<'_>,
) -> Result<(), AppError> {
    let scan_outcome = scan::perform_scan(
        input.scanner,
        input.file_data,
        input.scan_enabled,
        input.scan_fail_open,
    )
    .await?;

    let scanned_at = Utc::now();
    let _storage_object = if scan_outcome.is_quarantined {
        db::quarantine_storage_object_tx(
            tx,
            access.workspace_id,
            input.storage_object_id,
            input.actual_size,
            Some(input.computed_checksum),
            input.scan_engine,
            &scan_outcome.quarantine_reason,
            scanned_at,
        )
        .await?
    } else {
        db::activate_storage_object_tx(
            tx,
            access.workspace_id,
            input.storage_object_id,
            input.actual_size,
            Some(input.computed_checksum),
            &scan_outcome.status,
            scanned_at,
        )
        .await?
    };

    db::complete_upload_session_tx(tx, access.workspace_id, input.upload_id, scanned_at).await?;

    crate::domains::quotas::record_file_uploaded_tx(
        tx,
        FileUploadedUsageInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: access.auth.principal_id,
            storage_object_id: input.storage_object_id,
            upload_id: input.upload_id,
            size_bytes: input.actual_size,
            ip: input.ip,
            user_agent: input.user_agent,
        },
    )
    .await?;

    let audit_action = if scan_outcome.is_quarantined {
        "file.quarantined"
    } else {
        "file.uploaded"
    };

    insert_audit_event_tx(
        tx,
        AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: audit_action,
            target_type: "storage_object",
            target_id: Some(input.storage_object_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "upload_id": input.upload_id,
                "size_bytes": input.actual_size,
                "checksum": input.computed_checksum,
                "scan_status": scan_outcome.status,
                "quarantine_reason": if scan_outcome.is_quarantined { Some(&scan_outcome.quarantine_reason) } else { None },
                "tus": true,
            }),
        },
    )
    .await?;

    Ok(())
}
