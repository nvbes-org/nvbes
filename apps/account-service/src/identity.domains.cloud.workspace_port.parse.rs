use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::http::error::AppError;

pub(super) fn optional_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn parse_optional_datetime(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_datetime(value, field).map(Some)
    }
}

pub(super) fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| invalid_field(field))
}

pub(super) fn parse_optional_uuid(
    value: &str,
    field: &'static str,
) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

pub(super) fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| invalid_field(field))
}

pub(super) fn workspace_member_status(value: i32) -> &'static str {
    if value == crate::grpc_pb::nvbes::cloud::v1::WorkspaceMemberStatus::Active as i32 {
        "active"
    } else if value == crate::grpc_pb::nvbes::cloud::v1::WorkspaceMemberStatus::Suspended as i32 {
        "suspended"
    } else if value == crate::grpc_pb::nvbes::cloud::v1::WorkspaceMemberStatus::Removed as i32 {
        "removed"
    } else {
        "unspecified"
    }
}

fn invalid_field(field: &'static str) -> AppError {
    AppError::internal(
        "cloud_grpc_invalid_response",
        format!("Cloud returned an invalid {field}."),
    )
}
