use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateUploadResponse {
    pub upload_id: Uuid,
    pub storage_object: UploadObjectView,
    pub upload_url: SignedUploadUrlView,
    pub tus_url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompleteUploadResponse {
    pub upload_id: Uuid,
    pub storage_object: UploadObjectView,
    pub activated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CancelUploadResponse {
    pub upload_id: Uuid,
    pub storage_object_id: Uuid,
    pub cancelled_at: DateTime<Utc>,
}

pub struct CreateUploadInput {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub mime_type: String,
    pub expected_size_bytes: i64,
    pub expected_checksum: Option<String>,
}

pub struct CompleteUploadInput {
    pub size_bytes: i64,
    pub checksum: Option<String>,
}

pub struct CreateTusUploadInput {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub mime_type: String,
    pub upload_length: i64,
    pub expected_checksum: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TusUploadStatusView {
    pub upload_id: Uuid,
    pub upload_offset_bytes: i64,
    pub upload_length_bytes: i64,
    pub expires_at: DateTime<Utc>,
}

pub struct AppendTusChunkInput {
    pub upload_offset_bytes: i64,
    pub body: Vec<u8>,
}

pub struct AppendTusChunkResponse {
    pub upload_offset_bytes: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadObjectView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub object_type: String,
    pub status: String,
    pub scan_status: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub checksum: Option<String>,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignedUploadUrlView {
    pub url: String,
    pub method: &'static str,
    pub expires_at: DateTime<Utc>,
    pub required_headers: Vec<RequiredHeaderView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RequiredHeaderView {
    pub name: String,
    pub value: String,
}
