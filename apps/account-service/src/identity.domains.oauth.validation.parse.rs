use crate::http::error::AppError;

pub fn parse_step_up_level(value: &str) -> Result<String, AppError> {
    match value.trim().to_lowercase().as_str() {
        "aal1" | "aal2" | "aal3" => Ok(value.trim().to_lowercase()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Unsupported step-up level.",
        )),
    }
}

pub fn parse_client_policy_status(value: &str) -> Result<String, AppError> {
    match value.trim().to_lowercase().as_str() {
        "active" | "restricted" | "blocked" | "pending_approval" => Ok(value.trim().to_lowercase()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Unsupported client policy status.",
        )),
    }
}

pub fn parse_client_type(value: &str) -> Result<String, AppError> {
    match value.trim().to_lowercase().as_str() {
        "confidential" | "public" | "native" | "desktop" | "device" | "mobile" | "iot"
        | "service" => Ok(value.trim().to_lowercase()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Unsupported client type.",
        )),
    }
}
