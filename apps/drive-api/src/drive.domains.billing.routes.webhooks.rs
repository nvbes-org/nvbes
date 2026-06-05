use crate::domains::billing::service::BillingWebhookResponse;
use crate::{app::AppState, http::error::AppError};
use axum::{Json, Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use std::time::Instant;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new().route("/billing/webhooks", post(handle_webhook))
}

#[utoipa::path(
    post,
    path = "/billing/webhooks",
    tag = "billing",
    request_body(content = String, description = "Raw Stripe webhook payload", content_type = "application/json"),
    responses(
        (status = 200, description = "Webhook processed", body = BillingWebhookResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<BillingWebhookResponse>, AppError> {
    let started_at = Instant::now();
    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok());
    let result =
        crate::domains::billing::handle_webhook(&state.db, &state.config, signature, &body).await;

    match &result {
        Ok(_) => state.observability.record_billing_webhook(
            "stripe",
            "unknown",
            "success",
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_billing_webhook(
            "stripe",
            "unknown",
            "failure",
            started_at.elapsed(),
        ),
    }

    Ok(Json(result?))
}
