use crate::error::AppError;

pub(crate) fn validate_email(value: &str) -> Result<(), AppError> {
    let trimmed = value.trim();
    if trimmed.len() <= 254 && trimmed.contains('@') {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_compliance_email",
        "Email must look like a valid email address.",
    ))
}

pub(crate) fn validate_reason(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_compliance_reason",
        "Compliance action reason must contain between 8 and 500 characters.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_must_look_valid() {
        assert!(validate_email("privacy@example.com").is_ok());
        assert!(validate_email("invalid").is_err());
    }

    #[test]
    fn reason_must_be_operationally_useful() {
        assert!(validate_reason("ticket GDPR-123 approved").is_ok());
        assert!(validate_reason("short").is_err());
    }
}
