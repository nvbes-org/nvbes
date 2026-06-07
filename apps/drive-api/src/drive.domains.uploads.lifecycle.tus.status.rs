use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        uploads::{logic, queries, types::TusUploadStatusView},
    },
    http::error::AppError,
};

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
