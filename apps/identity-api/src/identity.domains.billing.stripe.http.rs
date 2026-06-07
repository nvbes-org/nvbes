use reqwest::StatusCode;

use crate::http::error::AppError;
use nvbes_billing::shared::form_encode;
use nvbes_core::config::AppConfig;

pub async fn stripe_post_form(
    config: &AppConfig,
    path: &str,
    fields: Vec<(String, String)>,
) -> Result<serde_json::Value, AppError> {
    let secret_key = config.stripe_secret_key.as_deref().ok_or_else(|| {
        AppError::conflict(
            "stripe_not_configured",
            "NVBES_STRIPE_SECRET_KEY must be configured before billing actions.",
        )
    })?;
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
        .map_err(|error| {
            AppError::internal(
                "stripe_request_failed",
                format!("Stripe request failed: {error}").as_str(),
            )
        })?;

    let status = response.status();
    let body = response.text().await.map_err(|error| {
        AppError::internal(
            "stripe_response_failed",
            format!("Stripe response failed: {error}").as_str(),
        )
    })?;

    if !status.is_success() {
        return Err(stripe_error(status, &body));
    }

    serde_json::from_str(&body).map_err(|error| {
        AppError::internal(
            "stripe_response_invalid",
            format!("Stripe response is not valid JSON: {error}").as_str(),
        )
    })
}

fn stripe_error(status: StatusCode, body: &str) -> AppError {
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

    if status == StatusCode::BAD_REQUEST {
        AppError::bad_request("stripe_request_rejected", &message)
    } else {
        AppError::conflict("stripe_request_rejected", &message)
    }
}
