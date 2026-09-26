use crate::models::BillingStateRecord;
use crate::provider::ProviderCode;
use crate::types::{CreatePortalInput, PortalSessionResponse};
use crate::{
    BillingRedirectUrlError, provider_code, provider_customer_id_for,
    provider_supports_external_portal, resolve_billing_redirect_url, stripe,
};
use nvbes_core::config::AppConfig;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortalSessionError {
    #[error("unknown_billing_provider")]
    UnknownBillingProvider,
    #[error("provider_portal_unavailable")]
    ProviderPortalUnavailable,
    #[error("missing_billing_customer")]
    MissingBillingCustomer,
    #[error(transparent)]
    Redirect(#[from] BillingRedirectUrlError),
    #[error(transparent)]
    Stripe(#[from] stripe::StripeProviderError),
}

pub async fn create_provider_portal_session(
    config: &AppConfig,
    record: &BillingStateRecord,
    input: CreatePortalInput,
) -> Result<PortalSessionResponse, PortalSessionError> {
    let billing_provider = provider_code(&record.billing_provider)
        .ok_or(PortalSessionError::UnknownBillingProvider)?;
    if !provider_supports_external_portal(billing_provider) {
        return Err(PortalSessionError::ProviderPortalUnavailable);
    }

    let customer_id = provider_customer_id_for(record, ProviderCode::Stripe)
        .ok_or(PortalSessionError::MissingBillingCustomer)?;
    let return_url = resolve_billing_redirect_url(
        input.return_url.as_deref(),
        &config.billing_default_portal_return_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
    )?;

    let session = stripe::create_stripe_portal_session(config, &customer_id, &return_url).await?;

    Ok(PortalSessionResponse {
        provider: ProviderCode::Stripe.as_str().to_string(),
        url: session.url,
    })
}

#[cfg(test)]
#[path = "portal_sessions.tests.rs"]
mod tests;
