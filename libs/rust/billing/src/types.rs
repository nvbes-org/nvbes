use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct CreateCheckoutInput {
    #[serde(alias = "planCode")]
    pub plan_code: String,
    #[serde(alias = "successUrl")]
    pub success_url: Option<String>,
    #[serde(alias = "cancelUrl")]
    pub cancel_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct CreatePortalInput {
    #[serde(alias = "returnUrl")]
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingOverviewResponse {
    pub workspace_id: Uuid,
    pub plan: PlanView,
    pub subscription: SubscriptionView,
    pub billing_account: BillingAccountView,
    pub entitlements: ProductEntitlementsView,
    pub invoice_estimate: InvoiceEstimateView,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CheckoutSessionResponse {
    pub provider: String,
    pub session_id: String,
    pub url: String,
    pub stripe_customer_id: String,
    pub stripe_price_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PortalSessionResponse {
    pub provider: String,
    pub url: String,
    pub stripe_customer_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingUsageResponse {
    pub workspace_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub storage: UsageLineView,
    pub seats: UsageLineView,
    pub bandwidth_out_bytes_month: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct InvoiceEstimateResponse {
    pub estimate: InvoiceEstimateView,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingWebhookResponse {
    pub provider_event_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PlanView {
    pub code: String,
    pub included_storage_gb: i32,
    pub included_users: i32,
    pub retention_days: i32,
    pub max_share_links: i32,
    pub audit_level: String,
    pub max_share_link_ttl_days: i32,
    pub monthly_price_cents: i64,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SubscriptionView {
    pub status: String,
    pub billing_provider: String,
    pub billing_customer_id: Option<String>,
    pub billing_subscription_id: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub trial_ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingAccountView {
    pub stripe_customer_id: Option<String>,
    pub billing_email: Option<String>,
    pub country: Option<String>,
    pub customer_type: String,
    pub vat_number: Option<String>,
    pub tax_exempt_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ProductEntitlementsView {
    pub can_upload: bool,
    pub can_create_share_links: bool,
    pub included_storage_bytes: i64,
    pub included_users: i32,
    pub max_share_links: i32,
    pub max_share_link_ttl_days: i32,
    pub audit_level: String,
    pub api_key_limit: i32,
    pub billing_locked: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct InvoiceEstimateView {
    pub workspace_id: Uuid,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub base_amount_cents: i64,
    pub storage_overage_amount_cents: i64,
    pub seat_overage_amount_cents: i64,
    pub estimated_amount_cents: i64,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageLineView {
    pub included_quantity: i64,
    pub used_quantity: i64,
    pub billable_quantity: i64,
    pub unit: String,
}
