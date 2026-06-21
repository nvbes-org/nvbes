use super::logic::{WebhookRetryDecision, classify_webhook_retry, resolve_webhook_workspace_id};
use serde_json::json;
use uuid::Uuid;

#[test]
fn resolve_webhook_workspace_id_prefers_metadata() {
    let metadata_workspace_id = Uuid::new_v4();
    let client_reference_id = Uuid::new_v4();
    let object = json!({
        "metadata": {
            "workspace_id": metadata_workspace_id.to_string()
        },
        "client_reference_id": client_reference_id.to_string(),
    });

    assert_eq!(
        resolve_webhook_workspace_id(&object),
        Some(metadata_workspace_id)
    );
}

#[test]
fn resolve_webhook_workspace_id_falls_back_to_client_reference() {
    let workspace_id = Uuid::new_v4();
    let object = json!({
        "client_reference_id": workspace_id.to_string(),
    });

    assert_eq!(resolve_webhook_workspace_id(&object), Some(workspace_id));
}

#[test]
fn resolve_webhook_workspace_id_returns_none_without_workspace_hint() {
    let object = json!({
        "metadata": {
            "plan_code": "pro"
        }
    });

    assert_eq!(resolve_webhook_workspace_id(&object), None);
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
fn classify_webhook_retry_treats_processed_as_duplicate() {
    assert_eq!(
        classify_webhook_retry(Some("processed")),
        WebhookRetryDecision::Duplicate
    );
}

#[test]
fn classify_webhook_retry_treats_missing_status_as_insert() {
    assert_eq!(classify_webhook_retry(None), WebhookRetryDecision::Insert);
}
