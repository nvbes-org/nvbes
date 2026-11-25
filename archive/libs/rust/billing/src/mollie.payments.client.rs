use crate::mollie::{
    MollieProviderError, build_mollie_payment_payload, checkout_from_mollie_payment,
    payment_from_mollie_response,
};
use crate::provider::{ProviderCheckout, ProviderCheckoutInput, ProviderPayment};
use nvbes_core::config::AppConfig;

pub async fn create_mollie_payment(
    config: &AppConfig,
    input: &ProviderCheckoutInput,
) -> Result<ProviderCheckout, MollieProviderError> {
    let payload = build_mollie_payment_payload(input)?;
    let response = crate::mollie::mollie_post_json(config, "/v2/payments", payload).await?;
    checkout_from_mollie_payment(response)
}

pub async fn fetch_mollie_payment(
    config: &AppConfig,
    provider_payment_id: &str,
) -> Result<ProviderPayment, MollieProviderError> {
    let response =
        crate::mollie::mollie_get_json(config, &format!("/v2/payments/{provider_payment_id}"))
            .await?;
    payment_from_mollie_response(response)
}
