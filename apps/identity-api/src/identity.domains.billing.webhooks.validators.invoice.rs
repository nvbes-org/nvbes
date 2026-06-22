use crate::http::error::AppError;
use nvbes_billing::required_string;
use serde_json::Value;

pub fn ensure_invoice_failure_consistency(object: &Value) -> Result<(), AppError> {
    let status = object.get("status").and_then(Value::as_str);
    if !matches!(status, Some("open" | "uncollectible" | "past_due")) {
        return Err(AppError::bad_request(
            "webhook_invoice_status_mismatch",
            "Invoice payment failed webhook has an unexpected invoice status.",
        ));
    }

    let attempt_count = object.get("attempt_count").and_then(Value::as_i64);
    if attempt_count.is_none() {
        return Err(AppError::bad_request(
            "webhook_invoice_attempt_count_missing",
            "Invoice payment failed webhook is missing the attempt count.",
        ));
    }
    if attempt_count.is_some_and(|value| value <= 0) {
        return Err(AppError::bad_request(
            "webhook_invoice_attempt_count_invalid",
            "Invoice payment failed webhook has an invalid attempt count.",
        ));
    }

    if matches!(
        object.get("collection_method").and_then(Value::as_str),
        Some(value) if !matches!(value, "charge_automatically" | "send_invoice")
    ) {
        return Err(AppError::bad_request(
            "webhook_invoice_collection_method_mismatch",
            "Invoice payment failed webhook has an unexpected collection method.",
        ));
    }

    Ok(())
}

pub fn ensure_invoice_billing_reason_consistency(object: &Value) -> Result<(), AppError> {
    let billing_reason = object.get("billing_reason").and_then(Value::as_str);
    if !billing_reason.is_some_and(|value| value.starts_with("subscription_")) {
        return Err(AppError::bad_request(
            "webhook_invoice_billing_reason_mismatch",
            "Invoice payment failed webhook has an unexpected billing reason.",
        ));
    }

    Ok(())
}

pub fn ensure_invoice_payment_intent_consistency(object: &Value) -> Result<(), AppError> {
    let payment_intent = object.get("payment_intent").and_then(Value::as_str);
    if payment_intent.is_none() {
        return Err(AppError::bad_request(
            "webhook_invoice_payment_intent_missing",
            "Invoice payment failed webhook is missing the payment intent id.",
        ));
    }

    Ok(())
}

pub fn ensure_invoice_amount_consistency(object: &Value) -> Result<(), AppError> {
    let amount_due = object.get("amount_due").and_then(Value::as_i64);
    if amount_due.is_none() {
        return Err(AppError::bad_request(
            "webhook_invoice_amount_due_missing",
            "Invoice payment failed webhook is missing the amount due.",
        ));
    }
    if amount_due.is_some_and(|value| value <= 0) {
        return Err(AppError::bad_request(
            "webhook_invoice_amount_due_invalid",
            "Invoice payment failed webhook has an invalid amount due.",
        ));
    }

    Ok(())
}

pub fn ensure_invoice_payment_success_consistency(object: &Value) -> Result<(), AppError> {
    if !matches!(object.get("status").and_then(Value::as_str), Some("paid")) {
        return Err(AppError::bad_request(
            "webhook_invoice_status_mismatch",
            "Invoice payment succeeded webhook has an unexpected invoice status.",
        ));
    }

    let amount_paid = object.get("amount_paid").and_then(Value::as_i64);
    if amount_paid.is_none() {
        return Err(AppError::bad_request(
            "webhook_invoice_amount_paid_missing",
            "Invoice payment succeeded webhook is missing the amount paid.",
        ));
    }
    if amount_paid.is_some_and(|value| value < 0) {
        return Err(AppError::bad_request(
            "webhook_invoice_amount_paid_invalid",
            "Invoice payment succeeded webhook has an invalid amount paid.",
        ));
    }

    Ok(())
}

pub fn ensure_invoice_subscription_consistency(
    object: &Value,
    current_subscription_id: Option<&str>,
    current_subscription_status: Option<&str>,
) -> Result<(), AppError> {
    let invoice_subscription_id = required_string(object, "subscription").ok_or_else(|| {
        AppError::bad_request(
            "webhook_invoice_subscription_missing",
            "Invoice payment failed webhook is missing the subscription id.",
        )
    })?;

    match current_subscription_id {
        Some(current_subscription_id) => {
            if current_subscription_id != invoice_subscription_id {
                return Err(AppError::bad_request(
                    "webhook_invoice_subscription_mismatch",
                    "Invoice payment failed webhook does not match the current workspace subscription.",
                ));
            }
        }
        None => {
            if matches!(current_subscription_status, Some("canceled")) {
                return Err(AppError::bad_request(
                    "webhook_invoice_subscription_missing",
                    "Invoice payment failed webhook does not match any current workspace subscription.",
                ));
            }
        }
    }

    Ok(())
}
