use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

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
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub mfa_enabled: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
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
    pub name: String,
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
pub struct EnterpriseSecuritySignal {
    pub key: String,
    pub label: String,
    pub status: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, serde_json::Value>>,
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
pub struct EnterpriseBillingPlan {
    pub code: String,
    pub name: String,
    pub status: String,
    pub currency: String,
    pub monthly_price_cents: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseInvoice {
    pub id: String,
    pub status: String,
    pub amount_due_cents: i64,
    pub currency: String,
    pub issued_at: DateTime<Utc>,
    pub hosted_invoice_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseUsageMetric {
    pub key: String,
    pub label: String,
    pub value: i64,
    pub limit: Option<i64>,
    pub unit: String,
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
    pub available_roles: Vec<EnterpriseRole>,
    pub available_module_grants: Vec<EnterpriseModuleGrant>,
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
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSecurityResponse {
    pub signals: Vec<EnterpriseSecuritySignal>,
    pub mfa_required: bool,
    pub passkeys_enabled: bool,
    pub recovery_approval_required: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAuditEventsResponse {
    pub events: Vec<EnterpriseAuditEvent>,
    pub page: EnterprisePage,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseBillingResponse {
    pub plan: EnterpriseBillingPlan,
    pub invoices: Vec<EnterpriseInvoice>,
    pub billing_email: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseUsageResponse {
    pub metrics: Vec<EnterpriseUsageMetric>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseInvitationInput {
    pub emails: Vec<String>,
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub workspace_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAccessUpdateInput {
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub workspace_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAuditReasonInput {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSuspendInput {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseReactivateInput {
    pub reason: String,
    pub module_grants: Option<Vec<EnterpriseModuleGrant>>,
    pub workspace_ids: Option<Vec<Uuid>>,
}
