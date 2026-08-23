use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkerJobRecord {
    pub id: Uuid,
    pub job_type: String,
    pub idempotency_key: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
}

#[derive(Debug)]
pub struct RecoveredJobResult {
    pub recovered: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ExpiredUploadCandidate {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub storage_key: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TrashRootCandidate {
    pub id: Uuid,
    pub workspace_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivacyRequestPayload {
    pub request_type: String,
    pub user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub request_id: Uuid,
}

#[derive(Debug)]
pub struct DeleteImpact {
    pub deleted_object_count: i64,
    pub deleted_file_count: i64,
    pub released_storage_bytes: i64,
}

pub enum StorageObjectType {
    File,
    Folder,
}
