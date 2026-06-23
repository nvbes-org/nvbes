use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[path = "drive.domains.audit.service.csv.rs"]
mod csv;
#[path = "drive.domains.audit.service.query.rs"]
mod query;
#[cfg(test)]
#[path = "drive.domains.audit.service.tests.rs"]
mod tests;

pub use csv::events_to_csv;
pub use query::{export_events, list_events, record_event};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;
const EXPORT_LIMIT: i64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct ListAuditEventsInput {
    pub limit: Option<i64>,
    pub before_id: Option<Uuid>,
    pub action: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub geo_network_kind: Option<String>,
    pub min_geo_risk_score: Option<i64>,
    pub geo_risk_label: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventsResponse {
    pub workspace_id: Uuid,
    pub events: Vec<AuditEventView>,
    pub next_before: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditExportResponse {
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: &'static str,
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub geo_country_code: Option<String>,
    pub geo_network_kind: Option<String>,
    pub geo_risk_score: Option<i64>,
    pub geo_risk_labels: Vec<String>,
    pub metadata: Value,
    pub previous_event_hash: Option<String>,
    pub event_hash: String,
    pub created_at: DateTime<Utc>,
}

pub struct AuditRecordInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: Value,
}

pub(super) fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

pub(super) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
