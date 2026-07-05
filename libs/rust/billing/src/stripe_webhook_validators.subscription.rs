use serde_json::Value;
use uuid::Uuid;

use crate::stripe_webhook_processing::{
    BillingWebhookProcessingError, BillingWebhookProcessingResult,
};
use crate::timestamp_field;

pub fn ensure_subscription_update_consistency(
    object: &Value,
) -> BillingWebhookProcessingResult<()> {
    let status = object.get("status").and_then(Value::as_str);
    let known_status = matches!(
        status,
        Some("active")
            | Some("trialing")
            | Some("past_due")
            | Some("canceled")
            | Some("cancelled")
            | Some("incomplete")
            | Some("incomplete_expired")
            | Some("unpaid")
            | Some("paused")
    );
    if !known_status {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_status_unknown",
            "Subscription update contains an unknown status.",
        ));
    }

    let collection_method = object.get("collection_method").and_then(Value::as_str);
    if !matches!(
        collection_method,
        Some("charge_automatically" | "send_invoice")
    ) {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_collection_method_unknown",
            "Subscription update contains an unknown collection method.",
        ));
    }

    let cancel_at_period_end = object.get("cancel_at_period_end").and_then(Value::as_bool);
    if cancel_at_period_end.is_none() {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_cancel_at_period_end_missing",
            "Subscription update is missing the cancel-at-period-end flag.",
        ));
    }

    let canceled_at = timestamp_field(object, "canceled_at");
    if matches!(status, Some("canceled" | "incomplete_expired")) && canceled_at.is_none() {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_canceled_at_missing",
            "Subscription update is missing the cancellation timestamp.",
        ));
    }

    let current_period_start = timestamp_field(object, "current_period_start");
    let current_period_end = timestamp_field(object, "current_period_end");
    if matches!(status, Some("active" | "trialing" | "past_due"))
        && (current_period_start.is_none() || current_period_end.is_none())
    {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_period_missing",
            "Subscription update is missing the current billing period.",
        ));
    }
    if let (Some(start), Some(end)) = (current_period_start, current_period_end)
        && end < start
    {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_period_invalid",
            "Subscription update contains an invalid billing period.",
        ));
    }

    let latest_invoice = object.get("latest_invoice").and_then(Value::as_str);
    let billing_cycle_anchor = timestamp_field(object, "billing_cycle_anchor");
    let latest_invoice_present = latest_invoice.is_some();
    let billing_cycle_anchor_present = billing_cycle_anchor.is_some();
    if latest_invoice_present != billing_cycle_anchor_present {
        if latest_invoice_present {
            return Err(BillingWebhookProcessingError::bad_request(
                "webhook_subscription_billing_cycle_anchor_missing",
                "Subscription update is missing the billing cycle anchor.",
            ));
        }

        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_latest_invoice_missing",
            "Subscription update is missing the latest invoice id.",
        ));
    }

    Ok(())
}

pub fn ensure_subscription_price_consistency(object: &Value) -> BillingWebhookProcessingResult<()> {
    let price_id = object
        .pointer("/items/data/0/price/id")
        .and_then(Value::as_str);
    if price_id.is_none() {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_price_missing",
            "Subscription update is missing the first item price id.",
        ));
    }

    Ok(())
}

pub fn ensure_subscription_item_consistency(object: &Value) -> BillingWebhookProcessingResult<()> {
    let item_id = object.pointer("/items/data/0/id").and_then(Value::as_str);
    if item_id.is_none() {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_item_missing",
            "Subscription update is missing the first item id.",
        ));
    }

    Ok(())
}

pub fn ensure_subscription_quantity_consistency(
    object: &Value,
) -> BillingWebhookProcessingResult<()> {
    let quantity = object
        .pointer("/items/data/0/quantity")
        .and_then(Value::as_i64);
    if quantity.is_none() {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_quantity_missing",
            "Subscription update is missing the first item quantity.",
        ));
    }
    if quantity.is_some_and(|value| value <= 0) {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_quantity_invalid",
            "Subscription update has an invalid first item quantity.",
        ));
    }

    Ok(())
}

pub fn ensure_subscription_deleted_consistency(
    object: &Value,
    workspace_id: Uuid,
) -> BillingWebhookProcessingResult<()> {
    super::stripe_webhook_validators_workspace::ensure_subscription_workspace_consistency(
        object,
        workspace_id,
    )?;
    let status = object.get("status").and_then(Value::as_str);
    if !matches!(status, Some("canceled" | "incomplete_expired")) {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_deleted_state_mismatch",
            "Subscription deleted webhook has an unexpected status.",
        ));
    }

    let canceled_at = timestamp_field(object, "canceled_at");
    if canceled_at.is_none() {
        return Err(BillingWebhookProcessingError::bad_request(
            "webhook_subscription_deleted_missing_canceled_at",
            "Subscription deleted webhook is missing the cancellation timestamp.",
        ));
    }

    Ok(())
}
