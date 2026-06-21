use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use utoipa::ToSchema;

use super::{EnterpriseAuditEvent, EnterprisePage};

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
