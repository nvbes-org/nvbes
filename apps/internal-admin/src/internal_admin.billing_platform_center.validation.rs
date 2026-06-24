use crate::error::AppError;

pub(crate) fn validate_reason(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_billing_platform_reason",
        "Billing platform action reason must contain between 8 and 500 characters.",
    ))
}
