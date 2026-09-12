use serde_json::Value;
use uuid::Uuid;

use crate::{StripeWebhookEvent, metadata_workspace_id, parse_uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookRetryDecision {
    Insert,
    Duplicate,
    ReplayFailed,
}

pub fn resolve_webhook_workspace_id(object: &Value) -> Option<Uuid> {
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
    serde_json::json!({
        "hash": payload_hash,
        "event_id": event.id,
        "event_type": event.event_type,
        "object_id": payload.get("id").and_then(Value::as_str),
        "object_type": payload.get("object").and_then(Value::as_str),
        "livemode": payload.get("livemode").and_then(Value::as_bool),
        "created": payload.get("created").and_then(Value::as_i64),
    })
}

pub fn classify_webhook_retry(existing_status: Option<&str>) -> WebhookRetryDecision {
    match existing_status {
        Some("processed") => WebhookRetryDecision::Duplicate,
        Some("failed" | "received") => WebhookRetryDecision::ReplayFailed,
        _ => WebhookRetryDecision::Insert,
    }
}

#[cfg(test)]
mod tests {
    use super::{WebhookRetryDecision, classify_webhook_retry};

    #[test]
    fn classify_webhook_retry_treats_processed_as_duplicate() {
        assert_eq!(
            classify_webhook_retry(Some("processed")),
            WebhookRetryDecision::Duplicate
        );
    }

    #[test]
    fn classify_webhook_retry_treats_failed_as_replayable() {
        assert_eq!(
            classify_webhook_retry(Some("failed")),
            WebhookRetryDecision::ReplayFailed
        );
    }

    #[test]
    fn classify_webhook_retry_treats_received_as_replayable() {
        assert_eq!(
            classify_webhook_retry(Some("received")),
            WebhookRetryDecision::ReplayFailed
        );
    }

    #[test]
    fn classify_webhook_retry_treats_missing_status_as_insert() {
        assert_eq!(classify_webhook_retry(None), WebhookRetryDecision::Insert);
    }
}
