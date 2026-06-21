use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct QuotaResponse {
    pub workspace_id: Uuid,
    pub used_storage_bytes: i64,
    pub included_storage_bytes: i64,
    pub file_count: i64,
    pub bandwidth_out_bytes_month: i64,
    pub storage_usage_percent: f64,
    pub upload_blocked: bool,
    pub alerts: Vec<QuotaAlertView>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QuotaAlertView {
    pub code: &'static str,
    pub level: &'static str,
    pub threshold_percent: i64,
    pub message: &'static str,
}

pub struct FileUploadedUsageInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Uuid,
    pub storage_object_id: Uuid,
    pub upload_id: Uuid,
    pub size_bytes: i64,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

pub struct StorageReleasedUsageInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub target_id: Uuid,
    pub source: &'a str,
    pub released_storage_bytes: i64,
    pub deleted_file_count: i64,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

pub struct BandwidthOutUsageInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub storage_object_id: Uuid,
    pub source: &'a str,
    pub size_bytes: i64,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub idempotency_key: String,
}

pub struct UsageSnapshot {
    pub included_storage_bytes: i64,
    pub used_storage_bytes: i64,
    pub upload_allowed: bool,
}

pub struct StorageThresholdAuditInput<'a> {
    pub workspace_id: Uuid,
    pub before: &'a UsageSnapshot,
    pub after_used_storage_bytes: i64,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Uuid,
    pub storage_object_id: Uuid,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

pub struct AuditEventInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}
