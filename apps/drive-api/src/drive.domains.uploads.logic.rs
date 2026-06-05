use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::models::UploadSessionRecord;
use super::types::*;
use crate::{domains::uploads::models::UploadSessionStatus, http::error::AppError};

const MAX_UPLOAD_BYTES: i64 = 25 * 1024 * 1024;
const REQUIRED_CONTENT_ENCODING: &str = "identity";

pub fn validate_object_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Object name cannot be empty.",
        ));
    }

    if trimmed == "." || trimmed == ".." || trimmed.contains('/') || trimmed.contains('\0') {
        return Err(AppError::bad_request(
            "validation_failed",
            "Object name contains unsupported characters.",
        ));
    }

    Ok(trimmed.to_owned())
}

pub fn validate_mime_type(mime_type: &str) -> Result<String, AppError> {
    let trimmed = mime_type.trim();

    if trimmed.is_empty() || !trimmed.contains('/') {
        return Err(AppError::bad_request(
            "validation_failed",
            "Mime type is invalid.",
        ));
    }

    Ok(trimmed.to_owned())
}

pub fn validate_size(size_bytes: i64) -> Result<i64, AppError> {
    if size_bytes <= 0 {
        return Err(AppError::bad_request(
            "validation_failed",
            "File size must be greater than zero.",
        ));
    }

    if size_bytes > MAX_UPLOAD_BYTES {
        return Err(AppError::bad_request(
            "upload_too_large",
            format!(
                "File size exceeds the {} byte upload limit.",
                MAX_UPLOAD_BYTES
            ),
        ));
    }

    Ok(size_bytes)
}

pub fn max_upload_bytes() -> i64 {
    MAX_UPLOAD_BYTES
}

pub fn required_content_encoding() -> &'static str {
    REQUIRED_CONTENT_ENCODING
}

pub fn ensure_plain_content_encoding(content_encoding: Option<&str>) -> Result<(), AppError> {
    let Some(content_encoding) = content_encoding else {
        return Ok(());
    };
    let normalized = content_encoding.trim();
    if normalized.is_empty() || normalized.eq_ignore_ascii_case(REQUIRED_CONTENT_ENCODING) {
        return Ok(());
    }

    Err(AppError::bad_request(
        "unsupported_content_encoding",
        "Compressed uploads are not supported.",
    ))
}

pub fn validate_tus_offset(offset_bytes: i64) -> Result<i64, AppError> {
    if offset_bytes < 0 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Upload offset cannot be negative.",
        ));
    }

    Ok(offset_bytes)
}

pub fn normalize_checksum(checksum: Option<String>) -> Result<Option<String>, AppError> {
    let Some(checksum) = checksum else {
        return Ok(None);
    };

    let normalized = checksum.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }

    if !normalized.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Checksum must be a hexadecimal string.",
        ));
    }

    Ok(Some(normalized))
}

pub fn build_object_key(workspace_id: Uuid, storage_object_id: Uuid, upload_id: Uuid) -> String {
    format!("workspaces/{workspace_id}/objects/{storage_object_id}/{upload_id}")
}

pub fn build_tus_upload_url(workspace_id: Uuid, upload_id: Uuid) -> String {
    format!("/workspaces/{workspace_id}/uploads/{upload_id}")
}

pub async fn build_signed_upload_url(
    storage: &dyn nvbes_storage::ObjectStore,
    object_key: &str,
    content_type: &str,
    expected_size_bytes: i64,
    expires_at: DateTime<Utc>,
) -> Result<SignedUploadUrlView, AppError> {
    let expires = (expires_at - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::from_secs(900));

    let presigned = storage
        .presign_upload(object_key, Some(content_type), expected_size_bytes, expires)
        .await
        .map_err(|e| {
            AppError::internal("storage_error", format!("Failed to presign upload: {e}"))
        })?;

    Ok(SignedUploadUrlView {
        url: presigned.url,
        method: presigned.method,
        expires_at,
        required_headers: vec![
            RequiredHeaderView {
                name: "content-length".to_owned(),
                value: expected_size_bytes.to_string(),
            },
            RequiredHeaderView {
                name: "content-type".to_owned(),
                value: content_type.to_owned(),
            },
            RequiredHeaderView {
                name: "content-encoding".to_owned(),
                value: required_content_encoding().to_owned(),
            },
        ],
    })
}

pub fn ensure_upload_is_completable(upload: &UploadSessionRecord) -> Result<(), AppError> {
    if !matches!(upload.status, UploadSessionStatus::Pending) {
        return Err(AppError::conflict(
            "upload_not_pending",
            "Only pending uploads can be completed.",
        ));
    }

    if upload.expires_at <= Utc::now() {
        return Err(AppError::conflict(
            "upload_expired",
            "Upload session has expired.",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_size_rejects_values_above_processing_cap() {
        let result = validate_size(max_upload_bytes() + 1);

        assert!(result.is_err());
    }

    #[test]
    fn ensure_plain_content_encoding_accepts_identity_and_absent_values() {
        assert!(ensure_plain_content_encoding(None).is_ok());
        assert!(ensure_plain_content_encoding(Some("identity")).is_ok());
        assert!(ensure_plain_content_encoding(Some(" Identity ")).is_ok());
    }

    #[test]
    fn ensure_plain_content_encoding_rejects_gzip() {
        let result = ensure_plain_content_encoding(Some("gzip"));

        assert!(result.is_err());
    }
}

pub fn ensure_upload_is_appendable(upload: &UploadSessionRecord) -> Result<(), AppError> {
    ensure_upload_is_completable(upload)
}
