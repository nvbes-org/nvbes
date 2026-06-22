#[path = "identity.domains.billing.provider.mollie.http.rs"]
mod http;
#[path = "identity.domains.billing.provider.mollie.payments.rs"]
mod payments;
#[path = "identity.domains.billing.provider.mollie.webhooks.rs"]
mod webhooks;

use nvbes_billing::provider::ProviderCode;

pub use payments::{
    build_mollie_payment_payload, checkout_from_mollie_payment, create_mollie_payment,
    fetch_mollie_payment, payment_from_mollie_response,
};
pub use webhooks::{mollie_payment_webhook_id, verify_mollie_classic_webhook};

pub const MOLLIE_PROVIDER: ProviderCode = ProviderCode::Mollie;

pub fn public_provider_name() -> &'static str {
    "mollie"
}
