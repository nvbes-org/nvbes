use nvbes_billing::provider::ProviderCode;

pub const STRIPE_PROVIDER: ProviderCode = ProviderCode::Stripe;

pub fn public_provider_name() -> &'static str {
    "stripe"
}
