use crate::models::BillingStateRecord;
use serde::{Deserialize, Serialize};
use std::future::Future;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderCode {
    Stripe,
    Mollie,
}

pub fn provider_code(value: &str) -> Option<ProviderCode> {
    match value {
        "stripe" => Some(ProviderCode::Stripe),
        "mollie" => Some(ProviderCode::Mollie),
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
            if record.billing_provider == "stripe" {
                record
                    .provider_customer_id
                    .clone()
                    .or_else(|| record.billing_customer_id.clone())
            } else {
                None
            }
        }),
        ProviderCode::Mollie => {
            if record.billing_provider == "mollie" {
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

#[cfg(test)]
mod tests {
    use super::{
        ProviderCode, provider_code, provider_customer_id_for, provider_supports_external_portal,
    };
    use crate::models::BillingStateRecord;
    use uuid::Uuid;

    fn billing_record(provider: &str) -> BillingStateRecord {
        BillingStateRecord {
            workspace_id: Uuid::nil(),
            workspace_name: "Acme".to_string(),
            owner_principal_id: Uuid::nil(),
            owner_email: "owner@example.com".to_string(),
            trial_ends_at: None,
            plan_id: Uuid::nil(),
            plan_code: "team".to_string(),
            included_storage_gb: 10,
            included_users: 2,
            retention_days: 90,
            max_share_links: 25,
            audit_level: "standard".to_string(),
            max_share_link_ttl_days: 30,
            subscription_status: "active".to_string(),
            billing_provider: provider.to_string(),
            billing_customer_id: Some(format!("{provider}_billing")),
            billing_subscription_id: None,
            current_period_start: None,
            current_period_end: None,
            provider_customer_id: Some(format!("{provider}_provider")),
            stripe_customer_id: Some("stripe_legacy".to_string()),
            billing_email: None,
            country: None,
            customer_type: "b2b".to_string(),
            vat_number: None,
            tax_exempt_status: None,
            used_storage_bytes: 0,
            bandwidth_out_bytes_month: 0,
            active_user_count: 1,
        }
    }

    #[test]
    fn provider_customer_id_for_prefers_legacy_stripe_id_for_stripe() {
        let record = billing_record("mollie");

        assert_eq!(
            provider_customer_id_for(&record, ProviderCode::Stripe).as_deref(),
            Some("stripe_legacy")
        );
    }

    #[test]
    fn provider_customer_id_for_uses_current_provider_customer() {
        let record = billing_record("mollie");

        assert_eq!(
            provider_customer_id_for(&record, ProviderCode::Mollie).as_deref(),
            Some("mollie_provider")
        );
    }

    #[test]
    fn provider_customer_id_for_does_not_reuse_other_provider_customer() {
        let mut record = billing_record("mollie");
        record.stripe_customer_id = None;

        assert_eq!(
            provider_customer_id_for(&record, ProviderCode::Stripe),
            None
        );
    }

    #[test]
    fn provider_code_parses_supported_provider_codes() {
        assert_eq!(provider_code("stripe"), Some(ProviderCode::Stripe));
        assert_eq!(provider_code("mollie"), Some(ProviderCode::Mollie));
        assert_eq!(provider_code("unknown"), None);
    }

    #[test]
    fn provider_supports_external_portal_only_for_stripe() {
        assert!(provider_supports_external_portal(ProviderCode::Stripe));
        assert!(!provider_supports_external_portal(ProviderCode::Mollie));
    }
}
