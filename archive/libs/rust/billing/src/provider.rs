use crate::models::BillingStateRecord;
use serde::{Deserialize, Serialize};
use std::future::Future;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCode {
    Stripe,
    Mollie,
    Cb,
}

impl ProviderCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderCode::Stripe => "stripe",
            ProviderCode::Mollie => "mollie",
            ProviderCode::Cb => "cb",
        }
    }
}

pub const PROVIDER_CODES: &[&str] = &["stripe", "mollie", "cb"];

pub fn provider_codes() -> &'static [&'static str] {
    PROVIDER_CODES
}

pub fn provider_code(value: &str) -> Option<ProviderCode> {
    match value {
        value if value == ProviderCode::Stripe.as_str() => Some(ProviderCode::Stripe),
        value if value == ProviderCode::Mollie.as_str() => Some(ProviderCode::Mollie),
        value if value == ProviderCode::Cb.as_str() => Some(ProviderCode::Cb),
        _ => None,
    }
}

pub fn provider_supports_external_portal(provider: ProviderCode) -> bool {
    matches!(provider, ProviderCode::Stripe)
}

pub fn provider_customer_id_for(
    record: &BillingStateRecord,
    provider: ProviderCode,
) -> Option<String> {
    match provider {
        ProviderCode::Stripe => record.stripe_customer_id.clone().or_else(|| {
            if record.billing_provider == ProviderCode::Stripe.as_str() {
                record
                    .provider_customer_id
                    .clone()
                    .or_else(|| record.billing_customer_id.clone())
            } else {
                None
            }
        }),
        ProviderCode::Mollie => {
            if record.billing_provider == ProviderCode::Mollie.as_str() {
                record
                    .provider_customer_id
                    .clone()
                    .or_else(|| record.billing_customer_id.clone())
            } else {
                None
            }
        }
        ProviderCode::Cb => {
            if record.billing_provider == ProviderCode::Cb.as_str() {
                record
                    .provider_customer_id
                    .clone()
                    .or_else(|| record.billing_customer_id.clone())
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCustomerInput {
    pub tenant_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCustomer {
    pub provider: ProviderCode,
    pub provider_customer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCheckoutInput {
    pub tenant_id: String,
    pub provider_customer_id: String,
    pub plan_code: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub success_url: String,
    pub cancel_url: String,
    pub webhook_url: Option<String>,
    pub fraud_metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCheckout {
    pub provider: ProviderCode,
    pub checkout_id: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPortalInput {
    pub provider_customer_id: String,
    pub return_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPortalSession {
    pub provider: ProviderCode,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRefundInput {
    pub provider_payment_id: String,
    pub amount_minor: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRefund {
    pub provider: ProviderCode,
    pub provider_refund_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPaymentMethod {
    pub method_type: String,
    pub brand: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<i16>,
    pub exp_year: Option<i16>,
    pub funding: Option<String>,
    pub issuer_country: Option<String>,
    pub fingerprint: Option<String>,
    pub provider_payment_method_id: Option<String>,
    pub mandate_id: Option<String>,
    pub mandate_status: String,
    pub reusable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPayment {
    pub provider: ProviderCode,
    pub provider_payment_id: String,
    pub provider_customer_id: Option<String>,
    pub provider_subscription_id: Option<String>,
    pub plan_code: Option<String>,
    pub status: String,
    pub amount_minor: i64,
    pub currency: String,
    pub payment_method: Option<ProviderPaymentMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSubscriptionInput {
    pub provider_customer_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub interval: String,
    pub description: String,
    pub start_date: Option<String>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSubscription {
    pub provider: ProviderCode,
    pub provider_subscription_id: String,
    pub status: String,
    pub provider_customer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderWebhookInput {
    pub headers: Vec<(String, String)>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderWebhookEvent {
    pub provider: ProviderCode,
    pub provider_event_id: String,
    pub event_type: String,
    pub payload_hash: String,
    pub payload_summary: serde_json::Value,
    pub signature_valid: bool,
    pub raw_retention_class: String,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider_unavailable")]
    Unavailable,
    #[error("provider_invalid_request: {0}")]
    InvalidRequest(String),
    #[error("provider_signature_invalid")]
    SignatureInvalid,
    #[error("provider_response_invalid")]
    ResponseInvalid,
}

pub trait PaymentProvider {
    fn create_customer(
        &self,
        input: ProviderCustomerInput,
    ) -> impl Future<Output = Result<ProviderCustomer, ProviderError>> + Send;

    fn create_checkout(
        &self,
        input: ProviderCheckoutInput,
    ) -> impl Future<Output = Result<ProviderCheckout, ProviderError>> + Send;

    fn create_portal_session(
        &self,
        input: ProviderPortalInput,
    ) -> impl Future<Output = Result<ProviderPortalSession, ProviderError>> + Send;

    fn refund_payment(
        &self,
        input: ProviderRefundInput,
    ) -> impl Future<Output = Result<ProviderRefund, ProviderError>> + Send;

    fn fetch_payment(
        &self,
        provider_payment_id: &str,
    ) -> impl Future<Output = Result<ProviderPayment, ProviderError>> + Send;

    fn create_subscription(
        &self,
        input: ProviderSubscriptionInput,
    ) -> impl Future<Output = Result<ProviderSubscription, ProviderError>> + Send;

    fn verify_webhook(
        &self,
        input: ProviderWebhookInput,
    ) -> impl Future<Output = Result<ProviderWebhookEvent, ProviderError>> + Send;
}
