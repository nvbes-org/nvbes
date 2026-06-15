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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnterpriseSecurityStatus {
    Complete,
    Attention,
    Critical,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSecurityResponse {
    pub signals: Vec<EnterpriseSecuritySignal>,
    pub posture: EnterpriseSecurityPostureScore,
    pub mfa_required: bool,
    pub passkeys_enabled: bool,
    pub recovery_approval_required: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSecurityPostureScore {
    pub score: i64,
    pub max_score: i64,
    pub completed_weight: i64,
    pub status: EnterpriseSecurityStatus,
    pub completed_controls: i64,
    pub total_controls: i64,
    pub controls: Vec<EnterpriseSecurityPostureControl>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSecurityPostureControl {
    pub id: String,
    pub label: String,
    pub description: String,
    pub status: EnterpriseSecurityStatus,
    pub weight: i64,
    pub completed_weight: i64,
    pub recommendation: String,
    pub owner: String,
    pub evidence: String,
    pub action_label: String,
    pub action_path: String,
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
pub struct EnterpriseAdminElevationInput {
    pub duration_minutes: Option<i64>,
    pub reason: Option<String>,
    pub procedure_reference: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAuditReasonInput {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseBreakGlassInput {
    pub reason: String,
    pub procedure_reference: String,
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
