use crate::app::AppState;

use crate::domains::billing::service::{self, BillingWebhookResponse};
use crate::domains::billing::{db, provider_mollie};
use crate::http::error::AppError;
use crate::http::request::client_ip;
use axum::{Json, Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use nvbes_core::limiter::RateLimiter;
use std::time::Duration;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/webhooks/stripe", post(handle_stripe_webhook))
        .route("/webhooks/mollie", post(handle_mollie_webhook))
}

#[utoipa::path(
    post,
    path = "/webhooks/stripe",
    tag = "billing",
    request_body(content = String, description = "Raw Stripe webhook payload", content_type = "application/json"),
    responses(
        (status = 200, description = "Webhook processed", body = BillingWebhookResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn handle_stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<BillingWebhookResponse>, AppError> {
    enforce_stripe_webhook_rate_limit(&state.rate_limiter, &headers).await?;
    let signature_header = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok());

    let result = service::handle_webhook(
        &state.db,
        &state.redis,
        &state.config,
        &state.observability,
        signature_header,
        body.as_ref(),
    )
    .await;

    Ok(Json(result?))
}

#[utoipa::path(
    post,
    path = "/webhooks/mollie",
    tag = "billing",
    request_body(content = String, description = "Raw Mollie webhook payload", content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Webhook accepted", body = BillingWebhookResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn handle_mollie_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<BillingWebhookResponse>, AppError> {
    enforce_mollie_webhook_rate_limit(&state.rate_limiter, &headers).await?;
    let event = provider_mollie::verify_mollie_classic_webhook(body.as_ref())?;
    let record = db::record_provider_event(&state.db, &event, None).await?;

    Ok(Json(BillingWebhookResponse {
        provider_event_id: record.provider_event_id,
        status: "accepted".to_string(),
    }))
}

pub(crate) async fn enforce_stripe_webhook_rate_limit(
    limiter: &RateLimiter,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let key = client_ip(headers).unwrap_or_else(|| "unknown".to_string());
    limiter
        .check("billing_webhook_stripe", &key, 300, Duration::from_secs(60))
        .await?;
    Ok(())
}

pub(crate) async fn enforce_mollie_webhook_rate_limit(
    limiter: &RateLimiter,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let key = client_ip(headers).unwrap_or_else(|| "unknown".to_string());
    limiter
        .check("billing_webhook_mollie", &key, 300, Duration::from_secs(60))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    #[test]
    fn stripe_webhook_rate_limit_key_uses_trusted_client_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-nvbes-client-ip", "203.0.113.10".parse().unwrap());
        assert_eq!(
            crate::http::request::client_ip(&headers).as_deref(),
            Some("203.0.113.10")
        );
    }

    #[test]
    fn stripe_webhook_rate_limit_key_ignores_untrusted_forwarded_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", "198.51.100.42".parse().unwrap());
        assert_eq!(crate::http::request::client_ip(&headers).as_deref(), None);
    }

    #[test]
    fn stripe_webhook_rate_limit_key_falls_back_to_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(crate::http::request::client_ip(&headers).as_deref(), None);
    }
}
