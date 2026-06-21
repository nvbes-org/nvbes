pub use billing_shared::models::{BillingStateRecord, PlanRecord, StripePriceMapping};
pub use billing_shared::types::{
    BillingOverviewResponse, BillingUsageResponse, BillingWebhookResponse, CheckoutSessionResponse,
    CreateCheckoutInput, CreatePortalInput, InvoiceEstimateResponse, PlanView,
    PortalSessionResponse, ProductEntitlementsView, UsageLineView,
};
use nvbes_billing as billing_shared;
use serde::Serialize;
use uuid::Uuid;

// AuditEventInput stays local because it is specific to identity-api audit log shape.
#[derive(Debug, Serialize)]
pub(crate) struct AuditEventInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProviderEventRecord {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub provider: String,
    pub provider_event_id: String,
    pub event_type: String,
    pub status: String,
    pub signature_valid: bool,
    pub payload_summary: serde_json::Value,
}
