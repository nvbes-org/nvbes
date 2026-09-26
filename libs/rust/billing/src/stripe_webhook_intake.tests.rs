use serde_json::json;
use uuid::Uuid;

use super::{
    WebhookRetryDecision, classify_webhook_retry, resolve_webhook_workspace_id,
    summarize_webhook_payload,
};
use crate::stripe::StripeWebhookEvent;

#[test]
fn classify_webhook_retry_treats_processed_as_duplicate() {
    assert_eq!(
        classify_webhook_retry(Some("processed")),
        WebhookRetryDecision::Duplicate
    );
}

#[test]
fn classify_webhook_retry_treats_failed_and_received_as_replayable() {
    assert_eq!(
        classify_webhook_retry(Some("failed")),
        WebhookRetryDecision::ReplayFailed
    );
    assert_eq!(
        classify_webhook_retry(Some("received")),
        WebhookRetryDecision::ReplayFailed
    );
    assert_eq!(classify_webhook_retry(None), WebhookRetryDecision::Insert);
}

#[test]
fn resolve_webhook_workspace_id_reads_metadata_or_client_reference() {
    let workspace = Uuid::new_v4();
    let from_metadata = json!({
        "metadata": { "workspace_id": workspace.to_string() }
    });
    assert_eq!(
        resolve_webhook_workspace_id(&from_metadata),
        Some(workspace)
    );

    let from_reference = json!({ "client_reference_id": workspace.to_string() });
    assert_eq!(
        resolve_webhook_workspace_id(&from_reference),
        Some(workspace)
    );
}

#[test]
fn summarize_webhook_payload_includes_object_metadata() {
    let event = StripeWebhookEvent {
        id: "evt_1".into(),
        event_type: "checkout.session.completed".into(),
        livemode: true,
        data_object: json!({
            "id": "cs_1",
            "object": "checkout.session",
            "livemode": true,
            "created": 1_700_000_000_i64
        }),
    };
    let summary = summarize_webhook_payload(&event, "payload-hash".into(), &event.data_object);
    assert_eq!(summary["hash"], "payload-hash");
    assert_eq!(summary["event_id"], "evt_1");
    assert_eq!(summary["object_id"], "cs_1");
    assert_eq!(summary["livemode"], true);
}
