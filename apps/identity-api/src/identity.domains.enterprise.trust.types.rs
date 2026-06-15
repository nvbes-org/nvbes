use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::enterprise::types::EnterpriseAuditEvent;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseTrustCenterResponse {
    pub tenant: TrustCenterTenant,
    pub mfa: TrustCenterMfaStatus,
    pub sso: TrustCenterSsoStatus,
    pub verified_domains: Vec<TrustCenterDomain>,
    pub audit: TrustCenterAuditStatus,
    pub hosting_regions: Vec<TrustCenterHostingRegion>,
    pub dpa: TrustCenterDocument,
    pub subprocessors: Vec<TrustCenterSubprocessor>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterTenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub security_tier: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterMfaStatus {
    pub enabled: bool,
    pub active_members: i64,
    pub members_with_mfa: i64,
    pub active_factors: i64,
    pub passkey_factors: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterSsoStatus {
    pub enabled: bool,
    pub active_providers: i64,
    pub required_domains: i64,
    pub providers: Vec<TrustCenterSsoProvider>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterSsoProvider {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
    pub provider_family: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterDomain {
    pub id: Uuid,
    pub domain: String,
    pub verified: bool,
    pub sso_required: bool,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterAuditStatus {
    pub immutable: bool,
    pub recent_events: Vec<EnterpriseAuditEvent>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterHostingRegion {
    pub data_region: String,
    pub legal_jurisdiction: String,
    pub workspace_count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterDocument {
    pub name: String,
    pub status: String,
    pub version: String,
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrustCenterSubprocessor {
    pub name: String,
    pub service: String,
    pub data_categories: String,
    pub location: String,
    pub transfer_outside_eea: bool,
    pub transfer_safeguard: String,
}
