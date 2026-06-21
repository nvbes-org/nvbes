use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[path = "identity.domains.enterprise.types.operations.rs"]
mod operations;
#[path = "identity.domains.enterprise.types.security.rs"]
mod security;

pub use operations::{
    EnterpriseAccessUpdateInput, EnterpriseAdminElevationInput, EnterpriseAuditReasonInput,
    EnterpriseBreakGlassInput, EnterpriseInvitationInput, EnterpriseReactivateInput,
    EnterpriseSuspendInput,
};
pub use security::{
    EnterpriseAuditEventsResponse, EnterpriseBillingPlan, EnterpriseBillingResponse,
    EnterpriseInvoice, EnterpriseSecurityPostureControl, EnterpriseSecurityPostureScore,
    EnterpriseSecurityResponse, EnterpriseSecuritySignal, EnterpriseSecurityStatus,
    EnterpriseUsageMetric, EnterpriseUsageResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnterpriseRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnterpriseModuleGrant {
    Members,
    Workspaces,
    Developers,
    Policies,
    Security,
    Billing,
    Audit,
    Drive,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePage {
    pub cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub break_glass: Option<EnterpriseBreakGlassAccount>,
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub mfa_enabled: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseBreakGlassAccount {
    pub procedure_reference: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseInvitation {
    pub id: Uuid,
    pub email: String,
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub invited_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseWorkspaceSummary {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: Option<String>,
    pub member_count: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseDeveloperCredentialSummary {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub secret_last4: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_email: Option<String>,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePolicySummary {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub configuration: BTreeMap<String, serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAuditEvent {
    pub id: Uuid,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseOverviewMetric {
    pub key: String,
    pub label: String,
    pub value: i64,
    pub delta_percent: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseContextResponse {
    pub tenant_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub user_id: Uuid,
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub break_glass: Option<EnterpriseBreakGlassAccount>,
    pub admin_elevation: EnterpriseAdminElevationView,
    pub available_roles: Vec<EnterpriseRole>,
    pub available_module_grants: Vec<EnterpriseModuleGrant>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAdminElevationView {
    pub active: bool,
    pub role: Option<EnterpriseRole>,
    pub expires_at: Option<DateTime<Utc>>,
    pub step_up_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAdminElevationResponse {
    pub elevation: EnterpriseAdminElevationView,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseOverviewResponse {
    pub metrics: Vec<EnterpriseOverviewMetric>,
    pub security_signals: Vec<EnterpriseSecuritySignal>,
    pub recent_audit_events: Vec<EnterpriseAuditEvent>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseUsersResponse {
    pub users: Vec<EnterpriseUser>,
    pub invitations: Vec<EnterpriseInvitation>,
    pub roles: Vec<EnterpriseRole>,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub page: EnterprisePage,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseInvitationsResponse {
    pub invitations: Vec<EnterpriseInvitation>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAccessUpdateResponse {
    pub user: EnterpriseUser,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseWorkspacesResponse {
    pub workspaces: Vec<EnterpriseWorkspaceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<EnterprisePage>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseDevelopersResponse {
    pub credentials: Vec<EnterpriseDeveloperCredentialSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<EnterprisePage>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePoliciesResponse {
    pub policies: Vec<EnterprisePolicySummary>,
    pub session_policy: EnterpriseSessionPolicy,
    pub mfa_policy: EnterpriseMfaPolicy,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSessionPolicy {
    pub admin_session_ttl_hours: i64,
    pub recommended_admin_session_ttl_hours: i64,
    pub compliant: bool,
    pub step_up_required_for_admin_elevation: bool,
    pub source: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSessionPolicyInput {
    pub admin_session_ttl_hours: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseMfaPolicy {
    pub policy: String,
    pub recommended_policy: String,
    pub compliant: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseMfaPolicyInput {
    pub policy: String,
}
