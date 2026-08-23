use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::domains::files::models::{StorageObjectStatus, StorageObjectType};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "upload_session_status", rename_all = "snake_case")]
pub enum UploadSessionStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
}

#[derive(Debug, FromRow)]
pub struct StorageObjectRecord {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub object_type: StorageObjectType,
    pub name: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub checksum: Option<String>,
    pub status: StorageObjectStatus,
    pub scan_status: String,
    pub object_key: Option<String>,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct UploadSessionRecord {
    pub storage_object_id: Uuid,
    pub expected_size_bytes: i64,
    pub expected_checksum: Option<String>,
    pub upload_offset_bytes: i64,
    pub storage_multipart_upload_id: Option<String>,
    pub status: UploadSessionStatus,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct UploadPartRecord {
    pub part_number: i32,
    pub offset_bytes: i64,
    pub size_bytes: i64,
    pub etag: String,
}
