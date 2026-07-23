use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListSecurityEventsInput {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub geo_country_code: Option<String>,
    pub geo_source: Option<String>,
    pub geo_confidence: Option<String>,
    pub geo_network_kind: Option<String>,
    pub min_geo_risk_score: Option<i64>,
    pub geo_risk_label: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventsResponse {
    pub events: Vec<SecurityEventView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub summary: SecurityEventsSummary,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct SecurityEventsSummary {
    pub total_events: usize,
    pub high_risk_events: usize,
    pub by_network_kind: Vec<SecurityEventCount>,
    pub by_country: Vec<SecurityEventCount>,
    pub by_risk_label: Vec<SecurityEventCount>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventCount {
    pub key: String,
    pub count: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventView {
    pub id: Uuid,
    pub event_type: String,
    pub created_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: Option<String>,
    pub risk_score: Option<f64>,
    pub geo_country_code: Option<String>,
    pub geo_source: Option<String>,
    pub geo_confidence: Option<String>,
    pub geo_network_kind: Option<String>,
    pub geo_risk_score: Option<i64>,
    pub geo_risk_labels: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityExportResponse {
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: &'static str,
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RiskEventView {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub session_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub risk_score: f64,
    pub risk_factors: Value,
    pub decision: String,
    pub metadata: Value,
    pub geo_country_code: Option<String>,
    pub geo_source: Option<String>,
    pub geo_confidence: Option<String>,
    pub geo_network_kind: Option<String>,
    pub geo_risk_score: Option<i64>,
    pub geo_risk_labels: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "identity.domains.security.types.tests.rs"]
mod tests;
