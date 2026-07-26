use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Developer-owned sandbox attached to a tenant.
#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperSandboxTenantSummary {
    pub tenant_id: Uuid,
    pub sandbox_tenant_id: Uuid,
    pub sandbox_name: String,
    pub sandbox_slug: String,
    pub status: String,
    pub data_profile: String,
    pub reset_requested_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperSandboxResponse {
    pub sandbox: Option<DeveloperSandboxTenantSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertDeveloperSandboxInput {
    pub data_profile: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperHealthCheckSummary {
    pub id: Uuid,
    pub target_type: String,
    pub target_id: String,
    pub check_kind: String,
    pub status: String,
    pub summary: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperHealthChecksResponse {
    pub checks: Vec<DeveloperHealthCheckSummary>,
}

#[derive(Debug, Serialize)]
pub struct RunDeveloperHealthChecksResponse {
    pub checks: Vec<DeveloperHealthCheckSummary>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperHealthCheckSeed {
    pub target_type: String,
    pub target_id: String,
    pub check_kind: String,
    pub status: String,
    pub summary: String,
    pub metadata: Value,
}
