use crate::error::AppError;

pub(crate) fn validate_client_id(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (3..=100).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_developer_client_id",
        "Developer client ID must contain between 3 and 100 characters.",
    ))
}

pub(crate) fn validate_reason(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_developer_reason",
        "Developer action reason must contain between 8 and 500 characters.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_must_be_bounded() {
        assert!(validate_client_id("client_123").is_ok());
        assert!(validate_client_id("id").is_err());
    }

    #[test]
    fn reason_must_be_operationally_useful() {
        assert!(validate_reason("ticket DEV-123 approved").is_ok());
        assert!(validate_reason("short").is_err());
    }
}
