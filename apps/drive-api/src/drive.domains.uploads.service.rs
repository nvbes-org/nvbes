use sqlx::PgPool;
use uuid::Uuid;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

use super::core;
use super::lifecycle;
pub use super::types::{
    AppendTusChunkInput, AppendTusChunkResponse, CancelUploadResponse, CompleteUploadInput,
    CompleteUploadResponse, CreateTusUploadInput, CreateUploadInput, CreateUploadResponse,
    TusUploadStatusView,
};

pub async fn create_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreateUploadInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CreateUploadResponse, AppError> {
    core::create_upload(storage, db, access, input, ip, user_agent).await
}

pub async fn create_tus_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreateTusUploadInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CreateUploadResponse, AppError> {
    core::create_tus_upload(storage, db, access, input, ip, user_agent).await
}

pub async fn get_tus_upload_status(
    db: &PgPool,
    access: &WorkspaceAccess,
    upload_id: Uuid,
) -> Result<TusUploadStatusView, AppError> {
    lifecycle::get_tus_upload_status(db, access, upload_id).await
}

#[expect(
    clippy::too_many_arguments,
    reason = "Upload service keeps infrastructure and scan toggles explicit."
)]
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
    lifecycle::append_tus_chunk(
        storage,
        scanner,
        db,
        access,
        upload_id,
        input,
        ip,
        user_agent,
        scan_enabled,
        scan_fail_open,
        scan_engine,
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "Upload service keeps infrastructure and scan toggles explicit."
)]
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
    lifecycle::complete_upload(
        storage,
        scanner,
        db,
        access,
        upload_id,
        input,
        ip,
        user_agent,
        scan_enabled,
        scan_fail_open,
        scan_engine,
    )
    .await
}

pub async fn cancel_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    upload_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CancelUploadResponse, AppError> {
    lifecycle::cancel_upload(storage, db, access, upload_id, ip, user_agent).await
}
