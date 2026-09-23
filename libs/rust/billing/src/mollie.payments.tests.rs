use super::{
    build_mollie_payment_payload, checkout_from_mollie_payment, payment_from_mollie_response,
};
use crate::mollie::MollieProviderError;
use crate::provider::{ProviderCheckoutInput, ProviderCode};

fn checkout_input(
    amount_minor: i64,
    webhook_url: Option<String>,
    fraud_metadata: Vec<(String, String)>,
) -> ProviderCheckoutInput {
    ProviderCheckoutInput {
        tenant_id: "tenant_1".to_string(),
        provider_customer_id: "cst_1".to_string(),
        plan_code: Some("team".to_string()),
        amount_minor,
        currency: "EUR".to_string(),
        success_url: "https://app.example/success".to_string(),
        cancel_url: "https://app.example/cancel".to_string(),
        webhook_url,
        fraud_metadata,
    }
}

#[test]
fn mollie_payment_payload_uses_amount_redirect_and_webhook() {
    let payload = build_mollie_payment_payload(&checkout_input(
        1234,
        Some("https://api.example/webhook".to_string()),
        vec![("fraud_score".to_string(), "42".to_string())],
    ))
    .expect("payload should build");

    assert_eq!(payload["amount"]["value"], "12.34");
    assert_eq!(payload["metadata"]["plan_code"], "team");
    assert_eq!(payload["metadata"]["fraud_score"], "42");
    assert_eq!(payload["webhookUrl"], "https://api.example/webhook");
}

#[test]
fn mollie_payment_payload_omits_webhook_when_absent() {
    let payload = build_mollie_payment_payload(&checkout_input(100, None, vec![]))
        .expect("payload should build");
    assert!(payload.get("webhookUrl").is_none());
}

#[test]
fn mollie_payment_payload_rejects_non_positive_amount() {
    let err =
        build_mollie_payment_payload(&checkout_input(0, None, vec![])).expect_err("zero amount");
    assert!(matches!(
        err,
        MollieProviderError::InvalidRequest {
            code: "invalid_mollie_amount",
            ..
        }
    ));

    let err =
        build_mollie_payment_payload(&checkout_input(-5, None, vec![])).expect_err("negative");
    assert!(matches!(err, MollieProviderError::InvalidRequest { .. }));
}

#[test]
fn checkout_from_mollie_payment_requires_id_and_checkout_link() {
    let err = checkout_from_mollie_payment(serde_json::json!({
        "_links": { "checkout": { "href": "https://mollie.test/pay" } }
    }))
    .expect_err("missing id");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));

    let err = checkout_from_mollie_payment(serde_json::json!({ "id": "tr_1" }))
        .expect_err("missing checkout link");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));
}

#[test]
fn checkout_from_mollie_payment_maps_success() {
    let checkout = checkout_from_mollie_payment(serde_json::json!({
        "id": "tr_ok",
        "_links": { "checkout": { "href": "https://mollie.test/checkout" } }
    }))
    .expect("checkout maps");
    assert_eq!(checkout.provider, ProviderCode::Mollie);
    assert_eq!(checkout.checkout_id, "tr_ok");
    assert_eq!(checkout.url, "https://mollie.test/checkout");
}

#[test]
fn mollie_payment_status_maps_to_provider_payment() {
    let payment = payment_from_mollie_response(serde_json::json!({
        "id": "tr_123",
        "status": "paid",
        "customerId": "cst_123",
        "subscriptionId": "sub_123",
        "metadata": { "plan_code": "team" },
        "amount": { "currency": "EUR", "value": "12.34" },
        "mandateId": "mdt_1",
        "details": {
            "cardLabel": "Mastercard",
            "cardNumber": "**** **** **** 4242",
            "cardFingerprint": "fp_abc",
            "cardExpiryDate": "12/29",
            "cardFunding": "credit",
            "cardCountryCode": "NL"
        }
    }))
    .expect("payment should parse");

    assert_eq!(payment.status, "captured");
    assert_eq!(payment.provider_customer_id.as_deref(), Some("cst_123"));
    assert_eq!(payment.provider_subscription_id.as_deref(), Some("sub_123"));
    assert_eq!(payment.amount_minor, 1234);
    let method = payment.payment_method.expect("card details");
    assert_eq!(method.last4.as_deref(), Some("4242"));
    assert_eq!(method.brand.as_deref(), Some("mastercard"));
    assert_eq!(method.mandate_status, "valid");
    assert!(method.reusable);
}

#[test]
fn mollie_payment_status_mapping_covers_failure_states() {
    for (mollie_status, provider_status) in [
        ("authorized", "authorized"),
        ("failed", "failed"),
        ("expired", "failed"),
        ("canceled", "canceled"),
        ("refunded", "refunded"),
        ("charged_back", "disputed"),
        ("open", "pending"),
        ("pending", "pending"),
        ("weird", "pending"),
    ] {
        let payment = payment_from_mollie_response(serde_json::json!({
            "id": "tr_x",
            "status": mollie_status,
            "amount": { "currency": "EUR", "value": "1.00" }
        }))
        .expect("payment parses");
        assert_eq!(payment.status, provider_status);
    }
}

#[test]
fn mollie_payment_defaults_missing_status_to_pending() {
    let payment = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "amount": { "currency": "EUR", "value": "1.00" }
    }))
    .expect("payment parses");
    assert_eq!(payment.status, "pending");
}

#[test]
fn mollie_payment_rejects_invalid_amount_format() {
    let err = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid",
        "amount": { "currency": "EUR", "value": "12" }
    }))
    .expect_err("missing decimal separator");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));
}

#[test]
fn mollie_payment_rejects_non_numeric_amount_parts() {
    let err = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid",
        "amount": { "currency": "EUR", "value": "ab.00" }
    }))
    .expect_err("invalid major");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));

    let err = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid",
        "amount": { "currency": "EUR", "value": "12.xx" }
    }))
    .expect_err("invalid minor");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));
}

#[test]
fn mollie_payment_rejects_missing_amount_currency_or_value() {
    let err = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid"
    }))
    .expect_err("missing amount");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));

    let err = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid",
        "amount": { "value": "1.00" }
    }))
    .expect_err("missing currency");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));

    let err = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid",
        "amount": { "currency": "EUR" }
    }))
    .expect_err("missing value");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));

    let err = payment_from_mollie_response(serde_json::json!({
        "status": "paid",
        "amount": { "currency": "EUR", "value": "1.00" }
    }))
    .expect_err("missing id");
    assert!(matches!(err, MollieProviderError::InvalidResponse { .. }));
}

#[test]
fn mollie_payment_method_is_none_without_usable_card_details() {
    let payment = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "open",
        "amount": { "currency": "EUR", "value": "1.00" }
    }))
    .expect("payment parses");
    assert!(payment.payment_method.is_none());

    let payment = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "open",
        "amount": { "currency": "EUR", "value": "1.00" },
        "details": { "cardNumber": "12" }
    }))
    .expect("payment parses");
    assert!(payment.payment_method.is_none());
}

#[test]
fn mollie_payment_method_uses_fingerprint_when_mandate_absent() {
    let payment = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "failed",
        "amount": { "currency": "EUR", "value": "2.50" },
        "details": {
            "cardLabel": "Visa",
            "cardFingerprint": "fp_only",
            "cardExpiryDate": "01/2031",
            "cardNumber": "1111"
        }
    }))
    .expect("payment parses");
    let method = payment.payment_method.expect("method");
    assert_eq!(
        method.provider_payment_method_id.as_deref(),
        Some("fp_only")
    );
    assert_eq!(method.mandate_status, "unknown");
    assert!(!method.reusable);
    assert_eq!(method.exp_month, Some(1));
    assert_eq!(method.exp_year, Some(2031));
    assert_eq!(method.last4.as_deref(), Some("1111"));
}

#[test]
fn mollie_payment_method_tolerates_invalid_expiry_formats() {
    let payment = payment_from_mollie_response(serde_json::json!({
        "id": "tr_x",
        "status": "paid",
        "amount": { "currency": "EUR", "value": "1.00" },
        "details": {
            "cardFingerprint": "fp",
            "cardExpiryDate": "not-an-expiry"
        }
    }))
    .expect("payment parses");
    let method = payment.payment_method.expect("method");
    assert_eq!(method.exp_month, None);
    assert_eq!(method.exp_year, None);
    assert_eq!(method.mandate_status, "unknown");
}
