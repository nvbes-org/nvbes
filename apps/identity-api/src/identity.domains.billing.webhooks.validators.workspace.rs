use crate::http::error::AppError;
use nvbes_billing::{metadata_workspace_id, parse_uuid};
use serde_json::Value;
use uuid::Uuid;

pub fn ensure_subscription_workspace_consistency(
    object: &Value,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let metadata_workspace_id = metadata_workspace_id(object);
    if metadata_workspace_id.is_some_and(|value| value != workspace_id) {
        return Err(AppError::bad_request(
            "webhook_workspace_mismatch",
            "Subscription metadata does not match the mapped workspace.",
        ));
    }

    Ok(())
}

pub fn ensure_subscription_workspace_alignment(
    object: &Value,
    mapped_workspace_id: Uuid,
) -> Result<(), AppError> {
    let metadata_workspace_id = metadata_workspace_id(object);
    if metadata_workspace_id.is_some_and(|value| value != mapped_workspace_id) {
        return Err(AppError::bad_request(
            "webhook_workspace_mismatch",
            "Subscription metadata does not match the Stripe customer mapping.",
        ));
    }

    Ok(())
}

pub fn ensure_checkout_workspace_consistency(
    object: &Value,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let metadata_workspace_id = metadata_workspace_id(object);
    let client_reference_id = object
        .get("client_reference_id")
        .and_then(Value::as_str)
        .and_then(parse_uuid);

    if metadata_workspace_id.is_some_and(|value| value != workspace_id)
        || client_reference_id.is_some_and(|value| value != workspace_id)
    {
        return Err(AppError::bad_request(
            "webhook_workspace_mismatch",
            "Checkout session metadata does not match the mapped workspace.",
        ));
    }

    Ok(())
}

pub fn ensure_invoice_workspace_consistency(
    object: &Value,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let metadata_workspace_id = metadata_workspace_id(object);
    let subscription_workspace_id = object
        .get("subscription_details")
        .and_then(|subscription_details| subscription_details.get("metadata"))
        .and_then(|metadata| metadata.get("workspace_id"))
        .and_then(Value::as_str)
        .and_then(parse_uuid);

    if metadata_workspace_id.is_some_and(|value| value != workspace_id)
        || subscription_workspace_id.is_some_and(|value| value != workspace_id)
    {
        return Err(AppError::bad_request(
            "webhook_workspace_mismatch",
            "Invoice metadata does not match the mapped workspace.",
        ));
    }

    Ok(())
}
