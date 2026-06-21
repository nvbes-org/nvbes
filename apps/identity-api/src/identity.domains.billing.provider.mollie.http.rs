use crate::http::error::AppError;
use nvbes_core::config::AppConfig;
use reqwest::StatusCode;

pub async fn mollie_post_json(
    config: &AppConfig,
    path: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let api_key = mollie_api_key(config)?;
    let url = mollie_url(config, path);
    let client = nvbes_core::security::pinned_http_client();
    let request = client.post(url).bearer_auth(api_key).json(&payload);
    mollie_send(request).await
}

pub async fn mollie_get_json(
    config: &AppConfig,
    path: &str,
) -> Result<serde_json::Value, AppError> {
    let api_key = mollie_api_key(config)?;
    let url = mollie_url(config, path);
    let client = nvbes_core::security::pinned_http_client();
    let request = client.get(url).bearer_auth(api_key);
    mollie_send(request).await
}

fn mollie_api_key(config: &AppConfig) -> Result<&str, AppError> {
    config.mollie_api_key.as_deref().ok_or_else(|| {
        AppError::conflict(
            "mollie_not_configured",
            "NVBES_MOLLIE_API_KEY must be configured before Mollie billing actions.",
        )
    })
}

fn mollie_url(config: &AppConfig, path: &str) -> String {
    format!(
        "{}{}",
        config.mollie_api_base_url.trim_end_matches('/'),
        path
    )
}

async fn mollie_send(request: reqwest::RequestBuilder) -> Result<serde_json::Value, AppError> {
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|error| {
            AppError::internal(
                "mollie_request_failed",
                format!("Mollie request failed: {error}").as_str(),
            )
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|error| {
        AppError::internal(
            "mollie_response_failed",
            format!("Mollie response failed: {error}").as_str(),
        )
    })?;
    if !status.is_success() {
        return Err(mollie_error(status, &body));
    }
    serde_json::from_str(&body).map_err(|error| {
        AppError::internal(
            "mollie_response_invalid",
            format!("Mollie response is not valid JSON: {error}").as_str(),
        )
    })
}

fn mollie_error(status: StatusCode, body: &str) -> AppError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("detail")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Mollie returned HTTP {status}."));

    if status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST {
        AppError::bad_request("mollie_request_rejected", &message)
    } else {
        AppError::conflict("mollie_request_rejected", &message)
    }
}
