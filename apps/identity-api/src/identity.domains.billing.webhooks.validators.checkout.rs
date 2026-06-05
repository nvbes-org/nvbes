use crate::http::error::AppError;
use serde_json::Value;

pub fn ensure_checkout_mode_consistency(
    object: &Value,
    has_subscription: bool,
) -> Result<(), AppError> {
    let mode = object.get("mode").and_then(Value::as_str);
    let expected_mode = if has_subscription {
        "subscription"
    } else {
        "payment"
    };

    if mode.is_none() || mode.is_some_and(|value| value != expected_mode) {
        return Err(AppError::bad_request(
            "webhook_checkout_mode_mismatch",
            "Checkout session mode does not match the expected Stripe flow.",
        ));
    }

    Ok(())
}

pub fn ensure_checkout_session_status_consistency(object: &Value) -> Result<(), AppError> {
    let status = object.get("status").and_then(Value::as_str);
    if status != Some("complete") {
        return Err(AppError::bad_request(
            "webhook_checkout_status_mismatch",
            "Checkout session status does not match the expected Stripe flow.",
        ));
    }

    Ok(())
}

pub fn ensure_checkout_subscription_presence(
    object: &Value,
    has_subscription: bool,
) -> Result<(), AppError> {
    if object.get("mode").and_then(Value::as_str) == Some("subscription") && !has_subscription {
        return Err(AppError::bad_request(
            "webhook_checkout_subscription_missing",
            "Checkout session is missing a subscription id for subscription mode.",
        ));
    }

    Ok(())
}

pub fn ensure_checkout_payment_status_consistency(
    object: &Value,
    has_subscription: bool,
) -> Result<(), AppError> {
    let payment_status = object.get("payment_status").and_then(Value::as_str);
    let allowed_statuses: &[&str] = if has_subscription {
        &["paid", "no_payment_required"]
    } else {
        &["paid"]
    };

    if payment_status.is_none()
        || payment_status.is_some_and(|value| !allowed_statuses.contains(&value))
    {
        return Err(AppError::bad_request(
            "webhook_checkout_payment_status_mismatch",
            "Checkout session payment status does not match the expected Stripe flow.",
        ));
    }

    Ok(())
}
