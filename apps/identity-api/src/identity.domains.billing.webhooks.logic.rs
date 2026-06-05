use super::validators_workspace::{
    ensure_subscription_workspace_alignment, ensure_subscription_workspace_consistency,
};
use crate::http::error::AppError;
use nvbes_billing::{StripeWebhookEvent, metadata_workspace_id, parse_uuid};
use serde_json::{Value, json};

pub fn resolve_webhook_workspace_id(object: &Value) -> Option<uuid::Uuid> {
    metadata_workspace_id(object).or_else(|| {
        object
            .get("client_reference_id")
            .and_then(Value::as_str)
            .and_then(parse_uuid)
    })
}

pub fn summarize_webhook_payload(
    event: &StripeWebhookEvent,
    payload_hash: String,
    payload: &Value,
) -> Value {
    json!({
        "hash": payload_hash,
        "event_id": event.id,
        "event_type": event.event_type,
        "object_id": payload.get("id").and_then(Value::as_str),
        "object_type": payload.get("object").and_then(Value::as_str),
        "livemode": payload.get("livemode").and_then(Value::as_bool),
        "created": payload.get("created").and_then(Value::as_i64),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookRetryDecision {
    Insert,
    Duplicate,
    ReplayFailed,
}

pub fn classify_webhook_retry(existing_status: Option<&str>) -> WebhookRetryDecision {
    match existing_status {
        Some("processed" | "received") => WebhookRetryDecision::Duplicate,
        Some("failed") => WebhookRetryDecision::ReplayFailed,
        _ => WebhookRetryDecision::Insert,
    }
}

pub fn resolve_subscription_workspace_id(
    object: &Value,
    mapped_workspace_id: uuid::Uuid,
) -> Result<uuid::Uuid, AppError> {
    let workspace_id = resolve_webhook_workspace_id(object).unwrap_or(mapped_workspace_id);
    ensure_subscription_workspace_consistency(object, workspace_id)?;
    ensure_subscription_workspace_alignment(object, mapped_workspace_id)?;
    Ok(workspace_id)
}
