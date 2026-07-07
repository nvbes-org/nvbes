use axum::http::{HeaderMap, header};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::HashMap;

use crate::{domains::uploads::logic, http::error::AppError};

pub(super) const TUS_VERSION: &str = "1.0.0";
pub(super) const TUS_RESUMABLE: &str = "Tus-Resumable";
pub(super) const TUS_VERSION_HEADER: &str = "Tus-Version";
pub(super) const TUS_EXTENSION: &str = "Tus-Extension";
pub(super) const TUS_MAX_SIZE: &str = "Tus-Max-Size";
pub(super) const UPLOAD_LENGTH: &str = "Upload-Length";
pub(super) const UPLOAD_OFFSET: &str = "Upload-Offset";
const UPLOAD_METADATA: &str = "Upload-Metadata";
const TUS_PATCH_CONTENT_TYPE: &str = "application/offset+octet-stream";

pub(super) fn ensure_tus_version(headers: &HeaderMap) -> Result<(), AppError> {
    let version = required_header(headers, TUS_RESUMABLE)?;
    if version != TUS_VERSION {
        return Err(AppError::bad_request(
            "unsupported_tus_version",
            "Tus-Resumable must be 1.0.0.",
        ));
    }

    Ok(())
}

pub(super) fn ensure_tus_patch_content_type(headers: &HeaderMap) -> Result<(), AppError> {
    let content_type = required_header(headers, header::CONTENT_TYPE.as_str())?;
    if content_type != TUS_PATCH_CONTENT_TYPE {
        return Err(AppError::bad_request(
            "invalid_tus_content_type",
            "TUS PATCH requests must use application/offset+octet-stream.",
        ));
    }

    Ok(())
}

pub(super) fn ensure_supported_content_encoding(headers: &HeaderMap) -> Result<(), AppError> {
    let content_encoding = headers
        .get(header::CONTENT_ENCODING)
        .map(|value| {
            value.to_str().map_err(|e| {
                AppError::bad_request(
                    "invalid_content_encoding",
                    format!("Content-Encoding is invalid: {e}"),
                )
            })
        })
        .transpose()?;

    logic::ensure_plain_content_encoding(content_encoding)
}

pub(super) fn required_i64_header(headers: &HeaderMap, name: &str) -> Result<i64, AppError> {
    let value = required_header(headers, name)?;
    value.parse::<i64>().map_err(|e| {
        AppError::bad_request(
            "invalid_header",
            format!("{name} must be a valid integer: {e}"),
        )
    })
}

pub(super) fn parse_tus_metadata(headers: &HeaderMap) -> Result<HashMap<String, String>, AppError> {
    let Some(raw_metadata) = headers.get(UPLOAD_METADATA) else {
        return Ok(HashMap::new());
    };
    let raw_metadata = raw_metadata.to_str().map_err(|e| {
        AppError::bad_request(
            "invalid_upload_metadata",
            format!("Upload-Metadata is invalid: {e}"),
        )
    })?;
    let mut metadata = HashMap::new();

    for item in raw_metadata
        .split(',')
        .filter(|item| !item.trim().is_empty())
    {
        let mut parts = item.trim().splitn(2, ' ');
        let key = parts.next().unwrap_or_default().trim();
        let value = parts.next().unwrap_or_default().trim();

        if key.is_empty() || value.is_empty() {
            return Err(AppError::bad_request(
                "invalid_upload_metadata",
                "Upload-Metadata entries must use '<key> <base64-value>'.",
            ));
        }

        let decoded = STANDARD.decode(value).map_err(|e| {
            AppError::bad_request(
                "invalid_upload_metadata",
                format!("Upload-Metadata value for {key} is not valid base64: {e}"),
            )
        })?;
        let decoded = String::from_utf8(decoded).map_err(|e| {
            AppError::bad_request(
                "invalid_upload_metadata",
                format!("Upload-Metadata value for {key} is not valid UTF-8: {e}"),
            )
        })?;
        metadata.insert(key.to_owned(), decoded);
    }

    Ok(metadata)
}

pub(super) fn insert_header(
    headers: &mut HeaderMap,
    name: &str,
    value: &str,
) -> Result<(), AppError> {
    let name = axum::http::HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
        AppError::internal(
            "invalid_response_header",
            format!("Invalid header name: {e}"),
        )
    })?;
    let value = axum::http::HeaderValue::from_str(value).map_err(|e| {
        AppError::internal(
            "invalid_response_header",
            format!("Invalid header value: {e}"),
        )
    })?;
    headers.insert(name, value);
    Ok(())
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String, AppError> {
    headers
        .get(name)
        .ok_or_else(|| AppError::bad_request("missing_header", format!("{name} is required.")))?
        .to_str()
        .map(str::to_owned)
        .map_err(|e| AppError::bad_request("invalid_header", format!("{name} is invalid: {e}")))
}
