use super::env::{env_bool, env_or_default, optional_env};

pub struct BillingProviderEnv {
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub stripe_api_base_url: String,
    pub mollie_api_key: Option<String>,
    pub mollie_api_base_url: String,
    pub billing_mollie_enabled: bool,
    pub billing_external_provider_fallback_enabled: bool,
}

pub fn billing_provider_env() -> Result<BillingProviderEnv, String> {
    Ok(BillingProviderEnv {
        stripe_secret_key: optional_env("NVBES_STRIPE_SECRET_KEY"),
        stripe_webhook_secret: optional_env("NVBES_STRIPE_WEBHOOK_SECRET"),
        stripe_api_base_url: env_or_default(
            "NVBES_STRIPE_API_BASE_URL",
            std::env::var("NVBES_STRIPE_API_BASE_URL").ok(),
            "https://api.stripe.com",
            false,
        )?,
        mollie_api_key: optional_env("NVBES_MOLLIE_API_KEY"),
        mollie_api_base_url: env_or_default(
            "NVBES_MOLLIE_API_BASE_URL",
            std::env::var("NVBES_MOLLIE_API_BASE_URL").ok(),
            "https://api.mollie.com",
            false,
        )?,
        billing_mollie_enabled: env_bool("NVBES_MOLLIE_ENABLED", false),
        billing_external_provider_fallback_enabled: env_bool(
            "NVBES_BILLING_EXTERNAL_PROVIDER_FALLBACK_ENABLED",
            false,
        ),
    })
}
