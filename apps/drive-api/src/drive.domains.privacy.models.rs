use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "privacy_request_type", rename_all = "snake_case")]
pub enum PrivacyRequestType {
    AccountExport,
    AccountDelete,
    WorkspaceExport,
    WorkspaceDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "privacy_request_status", rename_all = "snake_case")]
pub enum PrivacyRequestStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PrivacyRequest {
    pub id: Uuid,
    pub request_type: PrivacyRequestType,
    pub status: PrivacyRequestStatus,
    pub subject_user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub requested_by: Option<Uuid>,
    pub requested_by_principal_id: Option<Uuid>,
    pub worker_job_id: Option<Uuid>,
    pub identity_verified: bool,
    pub legal_hold: bool,
    pub rejection_reason: Option<String>,
    pub result: Option<JsonValue>,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
