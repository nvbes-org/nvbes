use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storage_object_type", rename_all = "snake_case")]
pub enum StorageObjectType {
    File,
    Folder,
}

impl StorageObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storage_object_status", rename_all = "snake_case")]
pub enum StorageObjectStatus {
    Pending,
    Active,
    Trashed,
    Deleted,
    Quarantined,
}

impl StorageObjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Trashed => "trashed",
            Self::Deleted => "deleted",
            Self::Quarantined => "quarantined",
        }
    }
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
    pub status: StorageObjectStatus,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub trashed_at: Option<DateTime<Utc>>,
}

impl StorageObjectRecord {
    pub fn ensure_mutable(&self) -> Result<(), crate::http::error::AppError> {
        if matches!(self.status, StorageObjectStatus::Deleted) {
            return Err(crate::http::error::AppError::conflict(
                "object_deleted",
                "Deleted objects cannot be modified.",
            ));
        }
        Ok(())
    }

    pub fn validate_name(name: &str) -> Result<String, crate::http::error::AppError> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(crate::http::error::AppError::bad_request(
                "validation_failed",
                "Object name cannot be empty.",
            ));
        }

        if trimmed == "." || trimmed == ".." {
            return Err(crate::http::error::AppError::bad_request(
                "validation_failed",
                "Object name is not allowed.",
            ));
        }

        if trimmed.contains('/') || trimmed.contains('\0') {
            return Err(crate::http::error::AppError::bad_request(
                "validation_failed",
                "Object name contains unsupported characters.",
            ));
        }

        Ok(trimmed.to_owned())
    }
}

pub struct DownloadMetadata {
    pub object_type: StorageObjectType,
    pub status: StorageObjectStatus,
    pub object_key: Option<String>,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
}
