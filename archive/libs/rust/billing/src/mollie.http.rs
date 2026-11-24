use reqwest::StatusCode;

use crate::mollie::MollieProviderError;
use nvbes_core::config::AppConfig;

pub async fn mollie_post_json(
    config: &AppConfig,
    path: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, MollieProviderError> {
    let api_key = mollie_api_key(config)?;
    let request = crate::shared::provider_http_client(&config.mollie_api_base_url)
        .post(mollie_url(config, path))
        .bearer_auth(api_key)
        .json(&payload);
    mollie_send(request).await
}

pub async fn mollie_get_json(
    config: &AppConfig,
    path: &str,
) -> Result<serde_json::Value, MollieProviderError> {
    let api_key = mollie_api_key(config)?;
    let request = crate::shared::provider_http_client(&config.mollie_api_base_url)
        .get(mollie_url(config, path))
        .bearer_auth(api_key);
    mollie_send(request).await
}

fn mollie_api_key(config: &AppConfig) -> Result<&str, MollieProviderError> {
    config
        .mollie_api_key
        .as_deref()
        .ok_or(MollieProviderError::NotConfigured)
}

fn mollie_url(config: &AppConfig, path: &str) -> String {
    format!(
        "{}{}",
        config.mollie_api_base_url.trim_end_matches('/'),
        path
    )
}

async fn mollie_send(
    request: reqwest::RequestBuilder,
) -> Result<serde_json::Value, MollieProviderError> {
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|error| MollieProviderError::RequestFailed(error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| MollieProviderError::ResponseFailed(error.to_string()))?;
    if !status.is_success() {
        return Err(mollie_error(status, &body));
    }
    serde_json::from_str(&body).map_err(|_| MollieProviderError::InvalidResponse {
        code: "mollie_response_invalid",
        message: "Mollie response is not valid JSON.",
    })
}

fn mollie_error(status: StatusCode, body: &str) -> MollieProviderError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("detail")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Mollie returned HTTP {status}."));

    MollieProviderError::RequestRejected {
        status: status.as_u16(),
        message,
    }
}

#[cfg(test)]
#[path = "mollie.http.tests.rs"]
mod tests;
