use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessReviewCampaignStatus {
    Draft,
    Active,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessReviewItemType {
    Member,
    Role,
    ServiceAccount,
    #[serde(rename = "oauth_client")]
    OAuthClient,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessReviewItemDecision {
    Pending,
    Approved,
    Revoked,
    Changed,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewCampaignScopeInput {
    pub include_members: bool,
    pub include_roles: bool,
    pub include_service_accounts: bool,
    pub include_oauth_clients: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateAccessReviewCampaignInput {
    pub name: String,
    pub description: Option<String>,
    pub due_at: DateTime<Utc>,
    pub scope: AccessReviewCampaignScopeInput,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewDecisionInput {
    pub decision: AccessReviewItemDecision,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewCampaignSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: AccessReviewCampaignStatus,
    pub starts_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub pending_items: i64,
    pub approved_items: i64,
    pub revoked_items: i64,
    pub changed_items: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewItem {
    pub id: Uuid,
    pub item_type: AccessReviewItemType,
    pub subject_id: String,
    pub subject_label: String,
    pub workspace_id: Option<Uuid>,
    pub role: Option<String>,
    pub status: String,
    pub evidence: BTreeMap<String, serde_json::Value>,
    pub decision: AccessReviewItemDecision,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewCampaignDetail {
    pub campaign: AccessReviewCampaignSummary,
    pub items: Vec<AccessReviewItem>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewDecisionResponse {
    pub campaign: AccessReviewCampaignSummary,
    pub item: AccessReviewItem,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessReviewCampaignsResponse {
    pub campaigns: Vec<AccessReviewCampaignSummary>,
}
