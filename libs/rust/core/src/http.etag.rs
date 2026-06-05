use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::http::error::AppError;

pub fn resource_etag(resource_type: &str, id: Uuid, updated_at: DateTime<Utc>) -> String {
    format!(
        "\"{}:{}:{}\"",
        resource_type,
        id,
        updated_at
            .timestamp_nanos_opt()
            .unwrap_or_else(|| updated_at.timestamp_micros() * 1_000)
    )
}

pub fn insert_etag(headers: &mut HeaderMap, etag: &str) -> Result<(), AppError> {
    let value = HeaderValue::from_str(etag)
        .map_err(|_| AppError::internal("invalid_etag", "Generated ETag is invalid."))?;
    headers.insert(header::ETAG, value);
    Ok(())
}

pub fn if_match(headers: &HeaderMap) -> Result<Option<String>, AppError> {
    optional_header(headers, header::IF_MATCH)
}

pub fn ensure_if_match(headers: &HeaderMap, current_etag: &str) -> Result<(), AppError> {
    match if_match(headers)? {
        Some(value) if etag_list_matches(&value, current_etag) => Ok(()),
        Some(_) => Err(AppError::precondition_failed(
            "etag_mismatch",
            "Resource has changed since the client last read it.",
        )),
        None => Ok(()),
    }
}

pub fn not_modified_if_none_match(
    headers: &HeaderMap,
    current_etag: &str,
) -> Result<Option<StatusCode>, AppError> {
    let Some(value) = optional_header(headers, header::IF_NONE_MATCH)? else {
        return Ok(None);
    };

    if etag_list_matches(&value, current_etag) {
        return Ok(Some(StatusCode::NOT_MODIFIED));
    }

    Ok(None)
}

pub fn etag_list_matches(value: &str, current_etag: &str) -> bool {
    value
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate == current_etag)
}

fn optional_header(
    headers: &HeaderMap,
    name: header::HeaderName,
) -> Result<Option<String>, AppError> {
    headers
        .get(name)
        .map(|value| {
            value.to_str().map(str::to_owned).map_err(|_| {
                AppError::bad_request("invalid_header", "HTTP precondition header is invalid.")
            })
        })
        .transpose()
}
