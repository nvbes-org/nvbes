use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use super::{
    StripeProviderError, constant_time_eq, hmac_sha256_hex, metadata_workspace_id,
    parse_stripe_event, parse_stripe_signature_header, required_string, stripe_subscription_status,
    timestamp_field, verify_stripe_signature,
};
use crate::shared::STRIPE_WEBHOOK_TOLERANCE_SECONDS;

fn signed_header(secret: &str, payload: &[u8], timestamp: i64) -> String {
    let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(payload));
    let signature = hmac_sha256_hex(secret.as_bytes(), signed_payload.as_bytes());
    format!("t={timestamp},v1={signature}")
}

#[test]
fn stripe_provider_error_codes_and_messages_are_stable() {
    let err = StripeProviderError::NotConfigured;
    assert_eq!(err.code(), "stripe_not_configured");
    assert!(err.message().contains("NVBES_STRIPE_SECRET_KEY"));
    assert!(!err.is_bad_request());

    let rejected = StripeProviderError::RequestRejected {
        status: 400,
        message: "bad".into(),
    };
    assert!(rejected.is_bad_request());
    assert_eq!(rejected.code(), "stripe_request_rejected");
}

#[test]
fn parse_stripe_signature_header_extracts_timestamp_and_signatures() {
    assert!(parse_stripe_signature_header("t=123,v1=abc").is_some());
    assert!(parse_stripe_signature_header("v1=only").is_none());
    assert!(parse_stripe_signature_header("t=123").is_none());
}

#[test]
fn hmac_sha256_hex_normalizes_long_keys() {
    let long_key = "k".repeat(80);
    let short_key = "secret";
    let message = b"payload";
    assert_ne!(
        hmac_sha256_hex(long_key.as_bytes(), message),
        hmac_sha256_hex(short_key.as_bytes(), message)
    );
}

#[test]
fn constant_time_eq_rejects_length_mismatch() {
    assert!(!constant_time_eq(b"ab", b"abc"));
    assert!(constant_time_eq(b"same", b"same"));
}

#[test]
fn parse_stripe_event_requires_core_fields() {
    assert!(parse_stripe_event(b"not-json").is_none());
    let payload = br#"{"id":"evt_1","type":"invoice.paid","data":{"object":{"id":"in_1"}}}"#;
    let event = parse_stripe_event(payload).expect("event");
    assert_eq!(event.id, "evt_1");
    assert_eq!(event.event_type, "invoice.paid");
    assert!(!event.livemode);
}

#[test]
fn metadata_workspace_id_reads_metadata_and_client_reference() {
    let workspace = Uuid::new_v4();
    let object = json!({
        "metadata": { "workspace_id": workspace.to_string() }
    });
    assert_eq!(metadata_workspace_id(&object), Some(workspace));

    let object = json!({ "client_reference_id": workspace.to_string() });
    assert_eq!(metadata_workspace_id(&object), None);
}

#[test]
fn required_string_and_timestamp_field_parse_stripe_objects() {
    let object = json!({ "id": "cs_1", "created": 1_700_000_000_i64 });
    assert_eq!(required_string(&object, "id").as_deref(), Some("cs_1"));
    assert!(timestamp_field(&object, "created").is_some());
    assert!(timestamp_field(&object, "missing").is_none());
}

#[test]
fn stripe_subscription_status_maps_provider_states() {
    assert_eq!(stripe_subscription_status(Some("active")), "active");
    assert_eq!(stripe_subscription_status(Some("cancelled")), "canceled");
    assert_eq!(stripe_subscription_status(Some("unpaid")), "suspended");
    assert_eq!(stripe_subscription_status(Some("unknown")), "suspended");
}

#[test]
fn verify_stripe_signature_accepts_valid_signature() {
    let secret = "whsec_test_secret";
    let payload =
        br#"{"id":"evt_1","type":"checkout.session.completed","data":{"object":{"id":"cs_1"}}}"#;
    let header = signed_header(secret, payload, Utc::now().timestamp());
    assert!(verify_stripe_signature(secret, Some(&header), payload).is_ok());
}

#[test]
fn verify_stripe_signature_rejects_missing_invalid_and_stale_signatures() {
    let secret = "whsec_test_secret";
    let payload = br#"{"id":"evt_1"}"#;
    assert_eq!(
        verify_stripe_signature(secret, None, payload),
        Err("missing_stripe_signature")
    );
    assert_eq!(
        verify_stripe_signature(secret, Some("bad"), payload),
        Err("invalid_stripe_signature")
    );

    let header = signed_header(
        secret,
        payload,
        Utc::now().timestamp() - STRIPE_WEBHOOK_TOLERANCE_SECONDS - 1,
    );
    assert_eq!(
        verify_stripe_signature(secret, Some(&header), payload),
        Err("stale_stripe_signature")
    );
}
