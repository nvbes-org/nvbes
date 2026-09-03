use reqwest::StatusCode;

use super::StripeProviderError;
use crate::shared::form_encode;
use nvbes_core::config::AppConfig;

pub async fn stripe_post_form(
    config: &AppConfig,
    path: &str,
    fields: Vec<(String, String)>,
) -> Result<serde_json::Value, StripeProviderError> {
    let secret_key = config
        .stripe_secret_key
        .as_deref()
        .ok_or(StripeProviderError::NotConfigured)?;
    if secret_key.starts_with("sk_live_") || secret_key.starts_with("rk_live_") {
        return Err(StripeProviderError::LiveKeyRejected);
    }
    let url = format!(
        "{}{}",
        config.stripe_api_base_url.trim_end_matches('/'),
        path
    );
    let body = form_encode(fields);
    let client = nvbes_core::security::pinned_http_client();

    let request = client
        .post(url)
        .bearer_auth(secret_key)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body);
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|error| StripeProviderError::RequestFailed(error.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| StripeProviderError::ResponseFailed(error.to_string()))?;

    if !status.is_success() {
        return Err(stripe_error(status, &body));
    }

    serde_json::from_str(&body)
        .map_err(|error| StripeProviderError::ResponseInvalid(error.to_string()))
}

fn stripe_error(status: StatusCode, body: &str) -> StripeProviderError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Stripe returned HTTP {status}."));

    StripeProviderError::RequestRejected {
        status: status.as_u16(),
        message,
    }
}
