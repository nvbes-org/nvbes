use serde::{Deserialize, Serialize};
use std::future::Future;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderCode {
    Stripe,
    Mollie,
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
    pub amount_minor: i64,
    pub currency: String,
    pub success_url: String,
    pub cancel_url: String,
    pub webhook_url: Option<String>,
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
pub struct ProviderPayment {
    pub provider: ProviderCode,
    pub provider_payment_id: String,
    pub status: String,
    pub amount_minor: i64,
    pub currency: String,
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

    fn verify_webhook(
        &self,
        input: ProviderWebhookInput,
    ) -> impl Future<Output = Result<ProviderWebhookEvent, ProviderError>> + Send;
}
