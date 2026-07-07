use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::http::error::AppError;

pub(crate) fn parse_required_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|error| {
        AppError::internal(
            "enterprise_projection_invalid",
            format!("{field} from Enterprise is not a valid UUID: {error}"),
        )
    })
}

pub(crate) fn parse_optional_uuid(
    value: &str,
    field: &'static str,
) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_required_uuid(value, field).map(Some)
    }
}

pub(crate) fn parse_required_time(
    value: &str,
    field: &'static str,
) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_projection_invalid",
                format!("{field} from Enterprise is not an RFC3339 timestamp: {error}"),
            )
        })
}

pub(crate) fn parse_optional_time(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_required_time(value, field).map(Some)
    }
}

pub(crate) fn empty_to_option(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
