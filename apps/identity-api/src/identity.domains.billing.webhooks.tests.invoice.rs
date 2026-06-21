use super::{validators_invoice as invoice, validators_workspace as workspace};
use invoice::*;
use serde_json::json;
use uuid::Uuid;
use workspace::*;

#[test]
fn ensure_invoice_workspace_consistency_rejects_mismatch() {
    let workspace_id = Uuid::new_v4();
    let mismatched_workspace_id = Uuid::new_v4();
    let object = json!({
        "metadata": {
            "workspace_id": workspace_id.to_string()
        },
        "subscription_details": {
            "metadata": {
                "workspace_id": mismatched_workspace_id.to_string()
            }
        }
    });

    let error = ensure_invoice_workspace_consistency(&object, workspace_id)
        .expect_err("mismatched invoice workspace metadata should be rejected");

    assert_eq!(error.code, "webhook_workspace_mismatch");
}

#[test]
fn ensure_invoice_workspace_consistency_accepts_nominal_invoice_without_metadata() {
    let workspace_id = Uuid::new_v4();
    let object = json!({
        "customer": "cus_123",
        "status": "open",
    });

    assert!(ensure_invoice_workspace_consistency(&object, workspace_id).is_ok());
}

#[test]
fn ensure_invoice_failure_consistency_accepts_open_invoice() {
    let object = json!({
        "status": "open",
        "attempt_count": 1,
    });

    assert!(ensure_invoice_failure_consistency(&object).is_ok());
}

#[test]
fn ensure_invoice_failure_consistency_rejects_invalid_status() {
    let object = json!({
        "status": "draft",
        "attempt_count": 1,
    });

    let error = ensure_invoice_failure_consistency(&object)
        .expect_err("unexpected invoice status should be rejected");

    assert_eq!(error.code, "webhook_invoice_status_mismatch");
}

#[test]
fn ensure_invoice_failure_consistency_rejects_negative_attempt_count() {
    let object = json!({
        "status": "open",
        "attempt_count": -1,
    });

    let error = ensure_invoice_failure_consistency(&object)
        .expect_err("negative invoice attempt count should be rejected");

    assert_eq!(error.code, "webhook_invoice_attempt_count_invalid");
}

#[test]
fn ensure_invoice_failure_consistency_rejects_missing_attempt_count() {
    let object = json!({
        "status": "open",
    });

    let error = ensure_invoice_failure_consistency(&object)
        .expect_err("invoice payment failed should require an attempt count");

    assert_eq!(error.code, "webhook_invoice_attempt_count_missing");
}

#[test]
fn ensure_invoice_failure_consistency_rejects_zero_attempt_count() {
    let object = json!({
        "status": "open",
        "collection_method": "charge_automatically",
        "attempt_count": 0,
    });

    let error = ensure_invoice_failure_consistency(&object)
        .expect_err("zero invoice attempt count should be rejected");

    assert_eq!(error.code, "webhook_invoice_attempt_count_invalid");
}

#[test]
fn ensure_invoice_failure_consistency_rejects_unknown_collection_method() {
    let object = json!({
        "status": "open",
        "collection_method": "manual",
        "attempt_count": 1,
    });

    let error = ensure_invoice_failure_consistency(&object)
        .expect_err("unexpected invoice collection method should be rejected");

    assert_eq!(error.code, "webhook_invoice_collection_method_mismatch");
}

#[test]
fn ensure_invoice_failure_consistency_accepts_subscription_invoice_shape() {
    let object = json!({
        "status": "open",
        "attempt_count": 1,
        "billing_reason": "subscription_cycle",
        "payment_intent": "pi_123",
        "amount_due": 1234,
        "subscription": "sub_123",
    });

    assert!(ensure_invoice_failure_consistency(&object).is_ok());
    assert!(ensure_invoice_billing_reason_consistency(&object).is_ok());
    assert!(ensure_invoice_payment_intent_consistency(&object).is_ok());
    assert!(ensure_invoice_amount_consistency(&object).is_ok());
}

#[test]
fn ensure_invoice_billing_reason_consistency_accepts_subscription_cycle() {
    let object = json!({
        "billing_reason": "subscription_cycle",
    });

    assert!(ensure_invoice_billing_reason_consistency(&object).is_ok());
}

#[test]
fn ensure_invoice_billing_reason_consistency_rejects_one_off_invoice() {
    let object = json!({
        "billing_reason": "manual",
    });

    let error = ensure_invoice_billing_reason_consistency(&object)
        .expect_err("non-subscription invoice should be rejected");

    assert_eq!(error.code, "webhook_invoice_billing_reason_mismatch");
}

#[test]
fn ensure_invoice_payment_intent_consistency_rejects_missing_payment_intent() {
    let object = json!({
        "status": "open",
        "attempt_count": 1,
        "billing_reason": "subscription_cycle",
    });

    let error = ensure_invoice_payment_intent_consistency(&object)
        .expect_err("invoice payment failed should require payment_intent");

    assert_eq!(error.code, "webhook_invoice_payment_intent_missing");
}

#[test]
fn ensure_invoice_amount_consistency_rejects_missing_amount_due() {
    let object = json!({
        "status": "open",
        "attempt_count": 1,
        "billing_reason": "subscription_cycle",
        "payment_intent": "pi_123",
        "subscription": "sub_123",
    });

    let error = ensure_invoice_amount_consistency(&object)
        .expect_err("invoice payment failed should require amount_due");

    assert_eq!(error.code, "webhook_invoice_amount_due_missing");
}

#[test]
fn ensure_invoice_payment_success_consistency_accepts_paid_invoice() {
    let object = json!({
        "status": "paid",
        "amount_paid": 4680,
        "subscription": "sub_123",
    });

    assert!(ensure_invoice_payment_success_consistency(&object).is_ok());
}

#[test]
fn ensure_invoice_payment_success_consistency_rejects_unpaid_invoice() {
    let object = json!({
        "status": "open",
        "amount_paid": 0,
        "subscription": "sub_123",
    });

    let error = ensure_invoice_payment_success_consistency(&object)
        .expect_err("payment succeeded should require paid invoice status");

    assert_eq!(error.code, "webhook_invoice_status_mismatch");
}

#[test]
fn ensure_invoice_payment_success_consistency_rejects_missing_amount_paid() {
    let object = json!({
        "status": "paid",
        "subscription": "sub_123",
    });

    let error = ensure_invoice_payment_success_consistency(&object)
        .expect_err("payment succeeded should require amount_paid");

    assert_eq!(error.code, "webhook_invoice_amount_paid_missing");
}

#[test]
fn ensure_invoice_subscription_consistency_accepts_matching_subscription() {
    let object = json!({
        "subscription": "sub_123",
    });

    assert!(
        ensure_invoice_subscription_consistency(&object, Some("sub_123"), Some("active")).is_ok()
    );
}

#[test]
fn ensure_invoice_subscription_consistency_rejects_mismatch() {
    let object = json!({
        "subscription": "sub_123",
    });

    let error = ensure_invoice_subscription_consistency(&object, Some("sub_456"), Some("active"))
        .expect_err("invoice should reject a mismatched subscription");

    assert_eq!(error.code, "webhook_invoice_subscription_mismatch");
}

#[test]
fn ensure_invoice_subscription_consistency_accepts_missing_current_subscription_when_active() {
    let object = json!({
        "subscription": "sub_123",
    });

    assert!(ensure_invoice_subscription_consistency(&object, None, Some("active")).is_ok());
}

#[test]
fn ensure_invoice_subscription_consistency_accepts_missing_current_subscription_without_status() {
    let object = json!({
        "subscription": "sub_123",
    });

    assert!(ensure_invoice_subscription_consistency(&object, None, None).is_ok());
}

#[test]
fn ensure_invoice_subscription_consistency_rejects_missing_current_subscription_when_canceled() {
    let object = json!({
        "subscription": "sub_123",
    });

    let error = ensure_invoice_subscription_consistency(&object, None, Some("canceled"))
        .expect_err("invoice should reject an event without a current workspace subscription");

    assert_eq!(error.code, "webhook_invoice_subscription_missing");
}

#[test]
fn ensure_invoice_failure_nominal_shape_is_accepted() {
    let object = json!({
        "id": "in_123",
        "status": "open",
        "collection_method": "charge_automatically",
        "attempt_count": 1,
        "amount_due": 1234,
        "billing_reason": "subscription_cycle",
        "payment_intent": "pi_123",
        "subscription": "sub_123",
        "customer": "cus_123",
    });

    assert!(ensure_invoice_failure_consistency(&object).is_ok());
    assert!(ensure_invoice_billing_reason_consistency(&object).is_ok());
    assert!(ensure_invoice_payment_intent_consistency(&object).is_ok());
    assert!(ensure_invoice_amount_consistency(&object).is_ok());
}
