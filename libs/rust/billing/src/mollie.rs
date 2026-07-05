#[path = "mollie.customers.rs"]
mod customers;
#[path = "mollie.http.rs"]
mod http;
#[path = "mollie.payments.rs"]
mod payments;
#[path = "mollie.payments.client.rs"]
mod payments_client;
#[path = "mollie.subscriptions.rs"]
mod subscriptions;
#[path = "mollie.webhooks.rs"]
mod webhooks;

use thiserror::Error;

pub use customers::{
    build_mollie_customer_payload, create_mollie_customer, customer_from_mollie_response,
};
pub use http::{mollie_get_json, mollie_post_json};
pub use payments::{
    build_mollie_payment_payload, checkout_from_mollie_payment, payment_from_mollie_response,
};
pub use payments_client::{create_mollie_payment, fetch_mollie_payment};
pub use subscriptions::{
    build_mollie_subscription_payload, create_mollie_subscription,
    subscription_from_mollie_response,
};
pub use webhooks::{mollie_payment_webhook_id, verify_mollie_classic_webhook};

#[derive(Debug, Error)]
pub enum MollieProviderError {
    #[error("{code}: {message}")]
    InvalidRequest {
        code: &'static str,
        message: &'static str,
    },
    #[error("{code}: {message}")]
    InvalidResponse {
        code: &'static str,
        message: &'static str,
    },
    #[error("{code}: {message}")]
    InvalidWebhook {
        code: &'static str,
        message: &'static str,
    },
    #[error("mollie_not_configured")]
    NotConfigured,
    #[error("mollie_request_failed: {0}")]
    RequestFailed(String),
    #[error("mollie_response_failed: {0}")]
    ResponseFailed(String),
    #[error("mollie_request_rejected: {message}")]
    RequestRejected { status: u16, message: String },
}

impl MollieProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest { code, .. }
            | Self::InvalidResponse { code, .. }
            | Self::InvalidWebhook { code, .. } => code,
            Self::NotConfigured => "mollie_not_configured",
            Self::RequestFailed(_) => "mollie_request_failed",
            Self::ResponseFailed(_) => "mollie_response_failed",
            Self::RequestRejected { .. } => "mollie_request_rejected",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::InvalidRequest { message, .. }
            | Self::InvalidResponse { message, .. }
            | Self::InvalidWebhook { message, .. } => message,
            Self::NotConfigured => {
                "NVBES_MOLLIE_API_KEY must be configured before Mollie billing actions."
            }
            Self::RequestFailed(message)
            | Self::ResponseFailed(message)
            | Self::RequestRejected { message, .. } => message,
        }
    }

    pub fn is_bad_request(&self) -> bool {
        matches!(
            self,
            Self::InvalidRequest { .. }
                | Self::InvalidWebhook { .. }
                | Self::RequestRejected {
                    status: 400 | 422,
                    ..
                }
        )
    }
}

pub fn minor_units_to_mollie_amount(amount_minor: i64) -> String {
    format!("{}.{:02}", amount_minor / 100, amount_minor % 100)
}
