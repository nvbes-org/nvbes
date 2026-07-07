use axum::{Json, Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
use nvbes_core::limiter::RateLimiter;
use std::time::Duration;

use crate::{app::BillingAppState, http::error::AppError};

pub fn router() -> Router<BillingAppState> {
    Router::new()
        .route("/webhooks/stripe", post(handle_stripe_webhook))
        .route("/webhooks/mollie", post(handle_mollie_webhook))
}

pub async fn handle_stripe_webhook(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeResponse>, AppError> {
    enforce_stripe_webhook_rate_limit(&state.rate_limiter, &headers).await?;
    let signature_header = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok());

    let started_at = std::time::Instant::now();
    let response = nvbes_billing::stripe_webhook_intake::handle_stripe_webhook_intake(
        &state.db,
        &state.redis,
        state.config.stripe_webhook_secret.as_deref(),
        signature_header,
        body.as_ref(),
    )
    .await?;
    state.observability.record_billing_webhook(
        &response.provider,
        &response.event_type,
        &response.status,
        started_at.elapsed(),
    );
    Ok(Json(response))
}

pub async fn handle_mollie_webhook(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    enforce_mollie_webhook_rate_limit(&state.rate_limiter, &headers).await?;
    let started_at = std::time::Instant::now();
    let event = nvbes_billing::mollie::verify_mollie_classic_webhook(body.as_ref())?;
    nvbes_billing::jobs::enqueue_mollie_webhook_job(&state.redis, &event.provider_event_id)
        .await
        .map_err(|error| AppError::internal("mollie_webhook_enqueue_failed", error.to_string()))?;
    state.observability.record_billing_webhook(
        event.provider.as_str(),
        &event.event_type,
        "queued",
        started_at.elapsed(),
    );
    Ok(Json(serde_json::json!({
        "provider": event.provider.as_str(),
        "event_id": event.provider_event_id,
        "status": "queued",
    })))
}

async fn enforce_stripe_webhook_rate_limit(
    limiter: &RateLimiter,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let key = nvbes_core::http::client_ip::client_ip(headers).unwrap_or_else(|| "unknown".into());
    limiter
        .check("billing_webhook_stripe", &key, 300, Duration::from_secs(60))
        .await?;
    Ok(())
}

async fn enforce_mollie_webhook_rate_limit(
    limiter: &RateLimiter,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let key = nvbes_core::http::client_ip::client_ip(headers).unwrap_or_else(|| "unknown".into());
    limiter
        .check("billing_webhook_mollie", &key, 300, Duration::from_secs(60))
        .await?;
    Ok(())
}
