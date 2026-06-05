use super::{validators_checkout as checkout, validators_workspace as workspace};
use checkout::*;
use serde_json::json;
use uuid::Uuid;
use workspace::*;

#[test]
fn ensure_checkout_workspace_consistency_rejects_mismatch() {
    let workspace_id = Uuid::new_v4();
    let mismatched_workspace_id = Uuid::new_v4();
    let object = json!({
        "metadata": {
            "workspace_id": workspace_id.to_string()
        },
        "client_reference_id": mismatched_workspace_id.to_string(),
    });

    let error = ensure_checkout_workspace_consistency(&object, workspace_id)
        .expect_err("mismatched checkout workspace metadata should be rejected");

    assert_eq!(error.code, "webhook_workspace_mismatch");
}

#[test]
fn ensure_checkout_workspace_consistency_accepts_matching_subscription_checkout() {
    let workspace_id = Uuid::new_v4();
    let object = json!({
        "metadata": {
            "workspace_id": workspace_id.to_string()
        },
        "client_reference_id": workspace_id.to_string(),
        "mode": "subscription",
        "subscription": "sub_123",
    });

    assert!(ensure_checkout_workspace_consistency(&object, workspace_id).is_ok());
}

#[test]
fn ensure_checkout_mode_consistency_accepts_subscription_checkout() {
    let object = json!({
        "mode": "subscription",
        "subscription": "sub_123",
    });

    assert!(ensure_checkout_mode_consistency(&object, true).is_ok());
}

#[test]
fn ensure_checkout_mode_consistency_rejects_incorrect_mode_for_subscription_checkout() {
    let object = json!({
        "mode": "payment",
        "subscription": "sub_123",
    });

    let error = ensure_checkout_mode_consistency(&object, true)
        .expect_err("subscription checkout should reject mismatched mode");

    assert_eq!(error.code, "webhook_checkout_mode_mismatch");
}

#[test]
fn ensure_checkout_mode_consistency_rejects_missing_mode_for_subscription_checkout() {
    let object = json!({
        "subscription": "sub_123",
    });

    let error = ensure_checkout_mode_consistency(&object, true)
        .expect_err("subscription checkout should require a mode");

    assert_eq!(error.code, "webhook_checkout_mode_mismatch");
}

#[test]
fn ensure_checkout_session_status_consistency_accepts_complete_session() {
    let object = json!({
        "status": "complete",
    });

    assert!(ensure_checkout_session_status_consistency(&object).is_ok());
}

#[test]
fn ensure_checkout_session_status_consistency_rejects_open_session() {
    let object = json!({
        "status": "open",
    });

    let error = ensure_checkout_session_status_consistency(&object)
        .expect_err("checkout session should reject a non-complete status");

    assert_eq!(error.code, "webhook_checkout_status_mismatch");
}

#[test]
fn ensure_checkout_session_status_consistency_rejects_missing_status() {
    let object = json!({});

    let error = ensure_checkout_session_status_consistency(&object)
        .expect_err("checkout session should require a status");

    assert_eq!(error.code, "webhook_checkout_status_mismatch");
}

#[test]
fn ensure_checkout_payment_status_consistency_accepts_paid_subscription_checkout() {
    let object = json!({
        "mode": "subscription",
        "payment_status": "paid",
        "subscription": "sub_123",
    });

    assert!(ensure_checkout_payment_status_consistency(&object, true).is_ok());
}

#[test]
fn ensure_checkout_payment_status_consistency_rejects_incomplete_subscription_checkout() {
    let object = json!({
        "mode": "subscription",
        "payment_status": "unpaid",
        "subscription": "sub_123",
    });

    let error = ensure_checkout_payment_status_consistency(&object, true)
        .expect_err("subscription checkout should reject unpaid payment status");

    assert_eq!(error.code, "webhook_checkout_payment_status_mismatch");
}

#[test]
fn ensure_checkout_payment_status_consistency_rejects_missing_payment_status() {
    let object = json!({
        "mode": "subscription",
        "subscription": "sub_123",
    });

    let error = ensure_checkout_payment_status_consistency(&object, true)
        .expect_err("subscription checkout should require a payment status");

    assert_eq!(error.code, "webhook_checkout_payment_status_mismatch");
}

#[test]
fn ensure_checkout_subscription_presence_accepts_subscription_checkout() {
    let object = json!({
        "status": "complete",
        "mode": "subscription",
        "payment_status": "paid",
        "subscription": "sub_123",
    });

    assert!(ensure_checkout_subscription_presence(&object, true).is_ok());
}

#[test]
fn ensure_checkout_subscription_presence_rejects_subscription_checkout_without_subscription_id() {
    let object = json!({
        "status": "complete",
        "mode": "subscription",
        "payment_status": "paid",
    });

    let error = ensure_checkout_subscription_presence(&object, false)
        .expect_err("subscription checkout should require subscription id");

    assert_eq!(error.code, "webhook_checkout_subscription_missing");
}

#[test]
fn ensure_checkout_completed_payment_flow_accepts_nominal_shape() {
    let workspace_id = Uuid::new_v4();
    let object = json!({
        "status": "complete",
        "mode": "payment",
        "payment_status": "paid",
        "customer": "cus_123",
        "metadata": {
            "workspace_id": workspace_id.to_string()
        },
        "client_reference_id": workspace_id.to_string(),
    });

    assert!(ensure_checkout_session_status_consistency(&object).is_ok());
    assert!(ensure_checkout_workspace_consistency(&object, workspace_id).is_ok());
    assert!(ensure_checkout_mode_consistency(&object, false).is_ok());
    assert!(ensure_checkout_payment_status_consistency(&object, false).is_ok());
    assert!(ensure_checkout_subscription_presence(&object, false).is_ok());
}

#[test]
fn ensure_checkout_completed_subscription_flow_accepts_nominal_shape() {
    let workspace_id = Uuid::new_v4();
    let object = json!({
        "status": "complete",
        "mode": "subscription",
        "payment_status": "paid",
        "customer": "cus_123",
        "subscription": "sub_123",
        "metadata": {
            "workspace_id": workspace_id.to_string()
        },
        "client_reference_id": workspace_id.to_string(),
    });

    assert!(ensure_checkout_session_status_consistency(&object).is_ok());
    assert!(ensure_checkout_workspace_consistency(&object, workspace_id).is_ok());
    assert!(ensure_checkout_mode_consistency(&object, true).is_ok());
    assert!(ensure_checkout_payment_status_consistency(&object, true).is_ok());
    assert!(ensure_checkout_subscription_presence(&object, true).is_ok());
}
