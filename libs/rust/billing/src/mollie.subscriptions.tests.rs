use serde_json::json;

use super::{build_mollie_subscription_payload, subscription_from_mollie_response};
use crate::mollie::MollieProviderError;
use crate::provider::{ProviderCode, ProviderSubscriptionInput};
#[test]
fn mollie_subscription_payload_uses_amount_interval_start_date_and_webhook() {
    let payload = build_mollie_subscription_payload(&ProviderSubscriptionInput {
        provider_customer_id: "cst_123".to_string(),
        amount_minor: 2500,
        currency: "EUR".to_string(),
        interval: "1 month".to_string(),
        description: "nvbes monthly subscription".to_string(),
        start_date: Some("2026-08-01".to_string()),
        webhook_url: Some("https://api.example/webhooks/mollie".to_string()),
    })
    .expect("payload should build");

    assert_eq!(payload["amount"]["value"], "25.00");
    assert_eq!(payload["interval"], "1 month");
    assert_eq!(payload["startDate"], "2026-08-01");
    assert_eq!(payload["webhookUrl"], "https://api.example/webhooks/mollie");
}

#[test]
fn mollie_subscription_payload_rejects_invalid_amount_and_interval() {
    let err = build_mollie_subscription_payload(&ProviderSubscriptionInput {
        provider_customer_id: "cst".into(),
        amount_minor: 0,
        currency: "EUR".into(),
        interval: "1 month".into(),
        description: "x".into(),
        start_date: None,
        webhook_url: None,
    })
    .unwrap_err();
    assert!(matches!(
        err,
        MollieProviderError::InvalidRequest {
            code: "invalid_mollie_amount",
            ..
        }
    ));

    let err = build_mollie_subscription_payload(&ProviderSubscriptionInput {
        provider_customer_id: "cst".into(),
        amount_minor: 100,
        currency: "EUR".into(),
        interval: "   ".into(),
        description: "x".into(),
        start_date: None,
        webhook_url: None,
    })
    .unwrap_err();
    assert!(matches!(
        err,
        MollieProviderError::InvalidRequest {
            code: "invalid_mollie_interval",
            ..
        }
    ));
}

#[test]
fn mollie_subscription_response_extracts_subscription_id() {
    let subscription = subscription_from_mollie_response(
        "cst_123",
        json!({ "id": "sub_123", "status": "active" }),
    )
    .expect("subscription should parse");

    assert_eq!(subscription.provider, ProviderCode::Mollie);
    assert_eq!(subscription.provider_subscription_id, "sub_123");
    assert_eq!(subscription.provider_customer_id, "cst_123");
    assert_eq!(subscription.status, "active");
}

#[test]
fn mollie_subscription_response_rejects_missing_id() {
    let err =
        subscription_from_mollie_response("cst_123", json!({ "status": "active" })).unwrap_err();
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));
}
