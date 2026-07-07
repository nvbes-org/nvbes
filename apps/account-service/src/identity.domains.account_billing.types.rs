use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingOverview {
    pub workspace_id: String,
    pub plan_code: String,
    pub subscription_status: String,
    pub billing_provider: String,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub entitlements: AccountBillingEntitlements,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingEntitlements {
    pub included_storage_gb: i64,
    pub included_users: i64,
    pub retention_days: i64,
    pub max_share_links: i64,
    pub audit_level: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingPortalView {
    pub workspace_id: String,
    pub provider: String,
    pub invoices: Vec<AccountBillingInvoice>,
    pub payment_methods: Vec<AccountBillingPaymentMethod>,
    pub subscriptions: Vec<AccountBillingSubscription>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingInvoice {
    pub invoice_id: String,
    pub invoice_number: Option<String>,
    pub status: String,
    pub total_minor: i64,
    pub currency: String,
    pub issued_at: Option<String>,
    pub providers: Vec<AccountBillingProviderReference>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingPaymentMethod {
    pub payment_method_id: String,
    pub brand: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<i32>,
    pub exp_year: Option<i32>,
    pub providers: Vec<AccountBillingProviderReference>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingSubscription {
    pub subscription_id: String,
    pub plan_code: Option<String>,
    pub status: String,
    pub providers: Vec<AccountBillingProviderReference>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingProviderReference {
    pub provider: String,
    pub status: String,
    pub primary: bool,
    pub fallback_eligible: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountBillingSession {
    pub url: String,
    pub provider: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateAccountCheckoutRequest {
    pub plan_code: String,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateAccountBillingPortalRequest {
    pub return_url: Option<String>,
}
