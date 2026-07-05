use axum::{Json, Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
use nvbes_core::limiter::RateLimiter;
use std::time::Duration;

use crate::{app::BillingAppState, http::error::AppError};

pub fn router() -> Router<BillingAppState> {
    Router::new().route("/webhooks/stripe", post(handle_stripe_webhook))
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
