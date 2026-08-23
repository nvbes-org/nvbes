use crate::error::AppError;

pub(crate) fn validate_data_region(value: &str) -> Result<(), AppError> {
    match value.trim() {
        "eu" | "us" | "ch" | "apac" => Ok(()),
        _ => Err(AppError::bad_request(
            "invalid_data_region",
            "Data region must be one of eu, us, ch or apac.",
        )),
    }
}

pub(crate) fn validate_jurisdiction(value: &str) -> Result<(), AppError> {
    match value.trim() {
        "gdpr" | "ccpa" | "nfdap" | "global" => Ok(()),
        _ => Err(AppError::bad_request(
            "invalid_jurisdiction",
            "Jurisdiction must be one of gdpr, ccpa, nfdap or global.",
        )),
    }
}

pub(crate) fn validate_reason(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_region_reason",
        "Region action reason must contain between 8 and 500 characters.",
    ))
}

pub(crate) fn validate_exception_kind(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (3..=96).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_region_exception_kind",
        "Region exception kind must contain between 3 and 96 characters.",
    ))
}
