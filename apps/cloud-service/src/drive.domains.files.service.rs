use uuid::Uuid;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

use super::core;
use super::lifecycle;
use super::transfer;
pub use super::types::{
    CreateFolderInput, DeleteObjectResponse, DownloadObjectInput, DownloadObjectResponse,
    DownloadObjectStatus, DownloadUrlResponse, ListObjectsInput, ListObjectsResponse,
    MoveObjectInput, ObjectResponse, ObjectTypeFilter, RenameObjectInput, TrashListInput,
    TrashListResponse,
};

pub async fn list_objects(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: ListObjectsInput,
) -> Result<ListObjectsResponse, AppError> {
    core::list_objects(db, access, input).await
}

pub async fn create_folder(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: CreateFolderInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    core::create_folder(db, access, input, ip, user_agent).await
}

pub async fn list_trash(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: TrashListInput,
) -> Result<TrashListResponse, AppError> {
    lifecycle::list_trash(db, access, input).await
}

pub async fn rename_object(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: RenameObjectInput,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    core::rename_object(db, access, object_id, input, expected_etag, ip, user_agent).await
}

pub async fn move_object(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: MoveObjectInput,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    core::move_object(db, access, object_id, input, expected_etag, ip, user_agent).await
}

pub async fn trash_object(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    lifecycle::trash_object(db, access, object_id, ip, user_agent).await
}

pub async fn restore_object(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    lifecycle::restore_object(db, access, object_id, ip, user_agent).await
}

pub async fn delete_object(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<DeleteObjectResponse, AppError> {
    lifecycle::delete_object(db, access, object_id, expected_etag, ip, user_agent).await
}

pub async fn create_download_url(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<DownloadUrlResponse, AppError> {
    transfer::create_download_url(storage, db, access, object_id, ip, user_agent).await
}

pub async fn download_object(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: DownloadObjectInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<DownloadObjectResponse, AppError> {
    transfer::download_object(storage, db, access, object_id, input, ip, user_agent).await
}
