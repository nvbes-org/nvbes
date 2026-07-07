use crate::error::AppError;

pub(crate) fn validate_admin_mutation(amount_minor: i64, reason: &str) -> Result<(), AppError> {
    if amount_minor <= 0 {
        return Err(AppError::bad_request(
            "invalid_amount",
            "Billing admin amount must be greater than zero.",
        ));
    }
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Billing admin actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

pub(crate) fn validate_provider_code(provider: &str) -> Result<(), AppError> {
    if matches!(provider, "stripe" | "mollie") {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_billing_provider",
        "Billing provider must be stripe or mollie.",
    ))
}
