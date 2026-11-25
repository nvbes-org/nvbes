use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "billing_provider", rename_all = "snake_case")]
pub enum BillingProvider {
    Stripe,
    Mollie,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Incomplete,
    Suspended,
}

impl From<String> for SubscriptionStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "trialing" => SubscriptionStatus::Trialing,
            "active" => SubscriptionStatus::Active,
            "past_due" => SubscriptionStatus::PastDue,
            "canceled" => SubscriptionStatus::Canceled,
            "incomplete" => SubscriptionStatus::Incomplete,
            "suspended" => SubscriptionStatus::Suspended,
            _ => SubscriptionStatus::Active, // Default or handle error
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "customer_type", rename_all = "snake_case")]
pub enum CustomerType {
    B2b,
    B2c,
}

impl From<String> for CustomerType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "b2b" => CustomerType::B2b,
            "b2c" => CustomerType::B2c,
            _ => CustomerType::B2b,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "billing_adjustment_type", rename_all = "snake_case")]
pub enum BillingAdjustmentType {
    Credit,
    Refund,
    ManualAdjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "billing_webhook_status", rename_all = "snake_case")]
pub enum BillingWebhookStatus {
    Received,
    Processed,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plan {
    pub id: Uuid,
    pub code: String,
    pub included_storage_gb: i32,
    pub included_users: i32,
    pub retention_days: i32,
    pub max_share_links: i32,
    pub audit_level: String,
    pub max_share_link_ttl_days: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub plan_id: Uuid,
    pub status: SubscriptionStatus,
    pub billing_provider: BillingProvider,
    pub billing_customer_id: Option<String>,
    pub billing_subscription_id: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingAccount {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub provider: BillingProvider,
    pub stripe_customer_id: Option<String>,
    pub billing_email: Option<String>,
    pub country: Option<String>,
    pub customer_type: CustomerType,
    pub vat_number: Option<String>,
    pub tax_exempt_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageEvent {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub meter: String,
    pub quantity: i64,
    pub unit: String,
    pub occurred_at: DateTime<Utc>,
    pub source: String,
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageSnapshot {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub meter: String,
    pub quantity: i64,
    pub unit: String,
    pub billable_quantity: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceEstimate {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub estimated_amount_cents: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingAdjustment {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub adjustment_type: BillingAdjustmentType,
    pub amount_cents: i64,
    pub currency: String,
    pub reason: String,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingWebhookEvent {
    pub id: Uuid,
    pub provider: BillingProvider,
    pub provider_event_id: String,
    pub status: BillingWebhookStatus,
    pub received_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub signature_valid: bool,
    pub payload: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingStateRecord {
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub owner_principal_id: Uuid,
    pub owner_email: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub plan_id: Uuid,
    pub plan_code: String,
    pub included_storage_gb: i32,
    pub included_users: i32,
    pub retention_days: i32,
    pub max_share_links: i32,
    pub audit_level: String,
    pub max_share_link_ttl_days: i32,
    pub subscription_status: String,
    pub billing_provider: String,
    pub billing_customer_id: Option<String>,
    pub billing_subscription_id: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub provider_customer_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub billing_email: Option<String>,
    pub country: Option<String>,
    pub customer_type: String,
    pub vat_number: Option<String>,
    pub tax_exempt_status: Option<String>,
    pub used_storage_bytes: i64,
    pub bandwidth_out_bytes_month: i64,
    pub active_user_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProviderPriceMapping {
    pub provider_product_id: String,
    pub provider_price_id: String,
    pub stripe_product_id: String,
    pub stripe_price_id: String,
    pub country_code: Option<String>,
    pub pricing_region: Option<String>,
    pub currency: String,
    pub amount_minor: Option<i64>,
}

pub type StripePriceMapping = ProviderPriceMapping;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanRecord {
    pub plan_id: Uuid,
    pub code: String,
}

#[cfg(test)]
#[path = "models.tests.rs"]
mod tests;
