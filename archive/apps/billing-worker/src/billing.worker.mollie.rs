use anyhow::Context;
use nvbes_billing::provider::{ProviderPayment, ProviderSubscription, ProviderSubscriptionInput};
use nvbes_core::config::AppConfig;
use reqwest::StatusCode;

pub async fn fetch_mollie_payment(
    config: &AppConfig,
    provider_payment_id: &str,
) -> anyhow::Result<ProviderPayment> {
    let response = mollie_get_json(config, &format!("/v2/payments/{provider_payment_id}")).await?;
    nvbes_billing::mollie::payment_from_mollie_response(response)
        .context("Mollie payment response invalid")
}

pub async fn create_mollie_subscription(
    config: &AppConfig,
    input: &ProviderSubscriptionInput,
) -> anyhow::Result<ProviderSubscription> {
    let response = mollie_post_json(
        config,
        &format!("/v2/customers/{}/subscriptions", input.provider_customer_id),
        nvbes_billing::mollie::build_mollie_subscription_payload(input)
            .context("Mollie subscription request invalid")?,
    )
    .await?;
    nvbes_billing::mollie::subscription_from_mollie_response(&input.provider_customer_id, response)
        .context("Mollie subscription response invalid")
}

async fn mollie_post_json(
    config: &AppConfig,
    path: &str,
    payload: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let api_key = mollie_api_key(config)?;
    let request = nvbes_core::security::pinned_http_client()
        .post(mollie_url(config, path))
        .bearer_auth(api_key)
        .json(&payload);
    mollie_send(request).await
}

async fn mollie_get_json(config: &AppConfig, path: &str) -> anyhow::Result<serde_json::Value> {
    let api_key = mollie_api_key(config)?;
    let request = nvbes_core::security::pinned_http_client()
        .get(mollie_url(config, path))
        .bearer_auth(api_key);
    mollie_send(request).await
}

fn mollie_api_key(config: &AppConfig) -> anyhow::Result<&str> {
    config
        .mollie_api_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("NVBES_MOLLIE_API_KEY must be configured."))
}

fn mollie_url(config: &AppConfig, path: &str) -> String {
    format!(
        "{}{}",
        config.mollie_api_base_url.trim_end_matches('/'),
        path
    )
}

async fn mollie_send(request: reqwest::RequestBuilder) -> anyhow::Result<serde_json::Value> {
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .context("Mollie request failed")?;
    let status = response.status();
    let body = response.text().await.context("Mollie response failed")?;
    if !status.is_success() {
        return Err(mollie_error(status, &body));
    }
    serde_json::from_str(&body).context("Mollie response is not valid JSON")
}

fn mollie_error(status: StatusCode, body: &str) -> anyhow::Error {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("detail")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Mollie returned HTTP {status}."));

    anyhow::anyhow!("Mollie request rejected with HTTP {status}: {message}")
}
