use crate::http::error::AppError;
use nvbes_billing::stripe::StripeWebhookEvent;

pub fn parse_stripe_event(payload: &[u8]) -> Result<StripeWebhookEvent, AppError> {
    nvbes_billing::parse_stripe_event(payload).ok_or_else(|| {
        AppError::bad_request(
            "invalid_webhook_payload",
            "Webhook payload is not valid JSON or missing required fields.",
        )
    })
}
