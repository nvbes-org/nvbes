use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ListObjectsResponse {
    pub parent_id: Option<Uuid>,
    pub objects: Vec<StorageObjectView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TrashListResponse {
    pub objects: Vec<StorageObjectView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

pub struct TrashListInput {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub object_type: Option<ObjectTypeFilter>,
    pub name_prefix: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ObjectResponse {
    pub object: StorageObjectView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteObjectResponse {
    pub deleted_object_id: Uuid,
    pub deleted_object_count: i64,
    pub deleted_file_count: i64,
    pub released_storage_bytes: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DownloadUrlResponse {
    pub object_id: Uuid,
    pub download_url: SignedDownloadUrlView,
}

pub struct DownloadObjectInput {
    pub range_header: Option<String>,
}

pub struct DownloadObjectResponse {
    pub body: Vec<u8>,
    pub status: DownloadObjectStatus,
    pub size_bytes: i64,
    pub content_type: String,
    pub served_bytes: i64,
}

pub enum DownloadObjectStatus {
    Full,
    Partial { start: i64, end_inclusive: i64 },
    Unsatisfiable,
}

pub struct ListObjectsInput {
    pub parent_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub object_type: Option<ObjectTypeFilter>,
    pub name_prefix: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObjectTypeFilter {
    File,
    Folder,
}

impl ObjectTypeFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
        }
    }
}

pub struct CreateFolderInput {
    pub parent_id: Option<Uuid>,
    pub name: String,
}

pub struct RenameObjectInput {
    pub name: String,
}

pub struct MoveObjectInput {
    pub destination_parent_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignedDownloadUrlView {
    pub url: String,
    pub method: &'static str,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StorageObjectView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub object_type: String,
    pub name: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub status: String,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub trashed_at: Option<DateTime<Utc>>,
}

impl From<crate::domains::files::models::StorageObjectRecord> for StorageObjectView {
    fn from(record: crate::domains::files::models::StorageObjectRecord) -> Self {
        Self {
            id: record.id,
            workspace_id: record.workspace_id,
            parent_id: record.parent_id,
            object_type: record.object_type.as_str().to_string(),
            name: record.name,
            size_bytes: record.size_bytes,
            mime_type: record.mime_type,
            status: record.status.as_str().to_string(),
            created_by: record.created_by,
            created_by_principal_id: record.created_by_principal_id,
            created_at: record.created_at,
            updated_at: record.updated_at,
            trashed_at: record.trashed_at,
        }
    }
}

#[cfg(test)]
#[path = "drive.domains.files.types.tests.rs"]
mod tests;
