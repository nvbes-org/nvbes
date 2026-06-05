use super::super::logic::resolve_subscription_workspace_id;
use super::super::{validators_subscription as subscription, validators_workspace as workspace};
use serde_json::json;
use subscription::*;
use uuid::Uuid;
use workspace::*;

#[test]
fn ensure_subscription_workspace_consistency_accepts_matching_metadata() {
    let workspace_id = Uuid::new_v4();
    let object = json!({"metadata": {"workspace_id": workspace_id.to_string()}});
    assert!(ensure_subscription_workspace_consistency(&object, workspace_id).is_ok());
}

#[test]
fn ensure_subscription_workspace_consistency_rejects_mismatch() {
    let workspace_id = Uuid::new_v4();
    let object = json!({"metadata": {"workspace_id": Uuid::new_v4().to_string()}});
    let error = ensure_subscription_workspace_consistency(&object, workspace_id)
        .expect_err("mismatched workspace metadata should be rejected");
    assert_eq!(error.code, "webhook_workspace_mismatch");
}

#[test]
fn resolve_subscription_workspace_id_rejects_customer_mapping_mismatch() {
    let metadata_workspace_id = Uuid::new_v4();
    let mapped_workspace_id = Uuid::new_v4();
    let object = json!({"metadata": {"workspace_id": metadata_workspace_id.to_string()}});
    let error = resolve_subscription_workspace_id(&object, mapped_workspace_id)
        .expect_err("subscription customer mapping mismatch should be rejected");
    assert_eq!(error.code, "webhook_workspace_mismatch");
}

#[test]
fn resolve_subscription_workspace_id_falls_back_to_customer_mapping_without_metadata() {
    let mapped_workspace_id = Uuid::new_v4();
    let object = json!({"status": "active"});
    let workspace_id = resolve_subscription_workspace_id(&object, mapped_workspace_id)
        .expect("subscription workspace should fall back to customer mapping");
    assert_eq!(workspace_id, mapped_workspace_id);
}

#[test]
fn resolve_subscription_workspace_id_falls_back_to_customer_mapping_for_delete_events() {
    let mapped_workspace_id = Uuid::new_v4();
    let object = json!({"id": "sub_123", "customer": "cus_123", "status": "canceled"});
    let workspace_id = resolve_subscription_workspace_id(&object, mapped_workspace_id)
        .expect("subscription delete should fall back to customer mapping");
    assert_eq!(workspace_id, mapped_workspace_id);
}

#[test]
fn ensure_subscription_deleted_consistency_accepts_canceled_status() {
    let workspace_id = Uuid::new_v4();
    let object =
        json!({"customer": "cus_123", "status": "canceled", "canceled_at": 1_700_000_000_i64});
    assert!(ensure_subscription_deleted_consistency(&object, workspace_id).is_ok());
}

#[test]
fn ensure_subscription_deleted_consistency_rejects_active_status() {
    let workspace_id = Uuid::new_v4();
    let object =
        json!({"customer": "cus_123", "status": "active", "canceled_at": 1_700_000_000_i64});
    let error = ensure_subscription_deleted_consistency(&object, workspace_id)
        .expect_err("subscription delete should reject an active status");
    assert_eq!(error.code, "webhook_subscription_deleted_state_mismatch");
}

#[test]
fn ensure_subscription_deleted_consistency_rejects_missing_canceled_at() {
    let workspace_id = Uuid::new_v4();
    let object = json!({"customer": "cus_123", "status": "canceled"});
    let error = ensure_subscription_deleted_consistency(&object, workspace_id)
        .expect_err("subscription delete should require canceled_at");
    assert_eq!(
        error.code,
        "webhook_subscription_deleted_missing_canceled_at"
    );
}

#[test]
fn ensure_subscription_update_consistency_accepts_known_status_and_period() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "latest_invoice": "in_123",
        "billing_cycle_anchor": 1_700_000_000_i64,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
    });
    assert!(ensure_subscription_update_consistency(&object).is_ok());
}

#[test]
fn ensure_subscription_update_consistency_rejects_unknown_status() {
    let object = json!({"status": "migrating", "collection_method": "charge_automatically", "cancel_at_period_end": false});
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("unknown subscription status should be rejected");
    assert_eq!(error.code, "webhook_subscription_status_unknown");
}

#[test]
fn ensure_subscription_update_consistency_rejects_invalid_period() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "current_period_start": 1_700_086_400_i64,
        "current_period_end": 1_700_000_000_i64,
    });
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("invalid subscription period should be rejected");
    assert_eq!(error.code, "webhook_subscription_period_invalid");
}

#[test]
fn ensure_subscription_update_consistency_rejects_missing_period_for_active_status() {
    let object = json!({"status": "active", "collection_method": "charge_automatically", "cancel_at_period_end": false});
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("active subscription update should require billing period");
    assert_eq!(error.code, "webhook_subscription_period_missing");
}

#[test]
fn ensure_subscription_update_consistency_rejects_unknown_collection_method() {
    let object = json!({
        "status": "active",
        "collection_method": "manual",
        "cancel_at_period_end": false,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
    });
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("unknown collection method should be rejected");
    assert_eq!(error.code, "webhook_subscription_collection_method_unknown");
}

#[test]
fn ensure_subscription_update_consistency_rejects_missing_cancel_at_period_end() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
    });
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("subscription update should require cancel_at_period_end");
    assert_eq!(
        error.code,
        "webhook_subscription_cancel_at_period_end_missing"
    );
}

#[test]
fn ensure_subscription_update_consistency_rejects_missing_canceled_at_for_terminal_status() {
    let object = json!({
        "status": "canceled",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": true,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
    });
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("terminal subscription status should require canceled_at");
    assert_eq!(error.code, "webhook_subscription_canceled_at_missing");
}

#[test]
fn ensure_subscription_update_consistency_rejects_missing_latest_invoice() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "billing_cycle_anchor": 1_700_000_000_i64,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
    });
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("subscription update should require latest_invoice");
    assert_eq!(error.code, "webhook_subscription_latest_invoice_missing");
}

#[test]
fn ensure_subscription_update_consistency_rejects_missing_billing_cycle_anchor() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "latest_invoice": "in_123",
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
    });
    let error = ensure_subscription_update_consistency(&object)
        .expect_err("subscription update should require billing_cycle_anchor");
    assert_eq!(
        error.code,
        "webhook_subscription_billing_cycle_anchor_missing"
    );
}

#[test]
fn ensure_subscription_price_consistency_rejects_missing_first_item_price() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "latest_invoice": "in_123",
        "billing_cycle_anchor": 1_700_000_000_i64,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
        "items": {"data": []}
    });
    let error = ensure_subscription_price_consistency(&object)
        .expect_err("subscription update should require an item price");
    assert_eq!(error.code, "webhook_subscription_price_missing");
}

#[test]
fn ensure_subscription_item_consistency_rejects_missing_first_item_id() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "latest_invoice": "in_123",
        "billing_cycle_anchor": 1_700_000_000_i64,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
        "items": {"data": [{"price": {"id": "price_123"}}]}
    });
    let error = ensure_subscription_item_consistency(&object)
        .expect_err("subscription update should require an item id");
    assert_eq!(error.code, "webhook_subscription_item_missing");
}

#[test]
fn ensure_subscription_quantity_consistency_rejects_missing_first_item_quantity() {
    let object = json!({
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "latest_invoice": "in_123",
        "billing_cycle_anchor": 1_700_000_000_i64,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
        "items": {"data": [{"id": "si_123", "price": {"id": "price_123"}}]}
    });
    let error = ensure_subscription_quantity_consistency(&object)
        .expect_err("subscription update should require an item quantity");
    assert_eq!(error.code, "webhook_subscription_quantity_missing");
}

#[test]
fn ensure_subscription_update_nominal_shape_is_accepted() {
    let object = json!({
        "id": "sub_123",
        "customer": "cus_123",
        "status": "active",
        "collection_method": "charge_automatically",
        "cancel_at_period_end": false,
        "latest_invoice": "in_123",
        "billing_cycle_anchor": 1_700_000_000_i64,
        "current_period_start": 1_700_000_000_i64,
        "current_period_end": 1_700_086_400_i64,
        "items": {"data": [{"id": "si_123", "quantity": 1, "price": {"id": "price_123"}}]}
    });
    assert!(ensure_subscription_update_consistency(&object).is_ok());
    assert!(ensure_subscription_item_consistency(&object).is_ok());
    assert!(ensure_subscription_price_consistency(&object).is_ok());
    assert!(ensure_subscription_quantity_consistency(&object).is_ok());
}
