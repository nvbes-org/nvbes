use std::net::SocketAddr;

#[derive(Clone)]
pub struct BillingConfig {
    pub public_origin: Option<String>,
    pub account_authority: Option<crate::authorization::AccountAuthority>,
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub stripe_api_base_url: String,
    pub identity_public_key_pem: Option<String>,
    pub identity_token_issuer: Option<String>,
    pub identity_token_key_id: Option<String>,
    pub identity_resource_client_id: Option<String>,
    pub identity_resource_secret: Option<String>,
    pub metrics_token: Option<String>,
    pub operator_token: Option<String>,
    pub app_url: String,
}

impl BillingConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let account_authority = match (
            std::env::var("NVBES_BILLING_ACCOUNT_ORIGIN").ok(),
            std::env::var("NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET").ok(),
        ) {
            (None, None) => None,
            (Some(origin), Some(secret)) => Some(crate::authorization::AccountAuthority::new(
                &origin, &secret,
            )?),
            _ => anyhow::bail!(
                "Account authorization origin and credential must be configured together"
            ),
        };
        let bind_addr: SocketAddr = std::env::var("NVBES_BILLING_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()?;

        let database_url = std::env::var("NVBES_BILLING_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/nvbes_billing".to_string()
        });

        let stripe_secret_key = std::env::var("NVBES_STRIPE_SECRET_KEY")
            .unwrap_or_else(|_| "sk_test_dummy_key_for_testing".to_string());

        if stripe_secret_key.starts_with("sk_live_") || stripe_secret_key.starts_with("rk_live_") {
            anyhow::bail!("CRITICAL FINOPS VIOLATION: Stripe live keys are forbidden in V1");
        }

        let stripe_webhook_secret = std::env::var("NVBES_STRIPE_WEBHOOK_SECRET")
            .unwrap_or_else(|_| "whsec_test_dummy_secret".to_string());

        let stripe_api_base_url = std::env::var("NVBES_STRIPE_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.stripe.com".to_string());

        let identity_public_key_pem = std::env::var("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM").ok();
        let identity_token_issuer = std::env::var("NVBES_IDENTITY_TOKEN_ISSUER").ok();
        let identity_token_key_id = std::env::var("NVBES_IDENTITY_TOKEN_KEY_ID").ok();
        let identity_resource_client_id =
            std::env::var("NVBES_BILLING_IDENTITY_RESOURCE_CLIENT_ID").ok();
        let identity_resource_secret = std::env::var("NVBES_BILLING_IDENTITY_RESOURCE_SECRET").ok();
        let metrics_token = std::env::var("NVBES_BILLING_METRICS_TOKEN").ok();
        let operator_token = std::env::var("NVBES_BILLING_OPERATOR_TOKEN").ok();

        let app_url =
            std::env::var("NVBES_APP_URL").unwrap_or_else(|_| "https://nvbes.test".to_string());

        Ok(Self {
            public_origin: std::env::var("NVBES_BILLING_PUBLIC_ORIGIN").ok(),
            account_authority,
            bind_addr,
            database_url,
            stripe_secret_key,
            stripe_webhook_secret,
            stripe_api_base_url,
            identity_public_key_pem,
            identity_token_issuer,
            identity_token_key_id,
            identity_resource_client_id,
            identity_resource_secret,
            metrics_token,
            operator_token,
            app_url,
        })
    }
}
