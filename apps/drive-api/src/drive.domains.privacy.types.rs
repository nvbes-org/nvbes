use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PrivacyRequestResponse {
    pub request_id: Uuid,
    pub request_type: String,
    pub status: String,
    pub worker_job_id: Uuid,
    pub requested_by_principal_id: Uuid,
    pub message: String,
    pub guardrails: Vec<String>,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PrivacyRequestStatusResponse {
    pub request_id: Uuid,
    pub request_type: String,
    pub status: String,
    pub worker_job_id: Option<Uuid>,
    pub requested_by_principal_id: Option<Uuid>,
    pub rejection_reason: Option<String>,
    pub result: Option<Value>,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct PrivacyRequestDraft {
    pub request_type: &'static str,
    pub job_type: &'static str,
    pub subject_user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub requested_by: Uuid,
    pub requested_by_principal_id: Uuid,
    pub rls_context: nvbes_tenancy::RlsContext,
    pub payload: Value,
    pub message: &'static str,
}
