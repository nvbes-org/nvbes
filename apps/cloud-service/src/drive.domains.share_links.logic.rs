use chrono::{DateTime, Duration as ChronoDuration, Utc};
use uuid::Uuid;

use super::db::{PublicShareRecord, ShareLinkRecord, SharePolicy, ShareableObjectRecord};
use super::types::SignedPublicDownloadUrlView;
use crate::{domains::files::models::StorageObjectStatus, http::error::AppError};

pub fn normalize_expires_at(
    requested: Option<DateTime<Utc>>,
    policy: &SharePolicy,
) -> Result<DateTime<Utc>, AppError> {
    let now = Utc::now();
    let expires_at =
        requested.unwrap_or_else(|| now + ChronoDuration::days(i64::from(policy.default_ttl_days)));
    let max_expires_at = now + ChronoDuration::days(i64::from(policy.max_ttl_days));

    if expires_at <= now {
        return Err(AppError::bad_request(
            "validation_failed",
            "Share link expiration must be in the future.",
        ));
    }

    if expires_at > max_expires_at {
        return Err(AppError::bad_request(
            "validation_failed",
            "Share link expiration exceeds the workspace maximum TTL.",
        ));
    }

    Ok(expires_at)
}

pub fn normalize_max_downloads(max_downloads: Option<i32>) -> Result<Option<i32>, AppError> {
    match max_downloads {
        Some(value) if value <= 0 => Err(AppError::bad_request(
            "validation_failed",
            "max_downloads must be greater than zero when provided.",
        )),
        value => Ok(value),
    }
}

pub fn ensure_link_not_revoked_or_expired(link: &ShareLinkRecord) -> Result<(), AppError> {
    if link.revoked_at.is_some() {
        return Err(AppError::conflict(
            "share_link_revoked",
            "Share link is revoked.",
        ));
    }

    if link.expires_at <= Utc::now() {
        return Err(AppError::conflict(
            "share_link_expired",
            "Share link is expired.",
        ));
    }

    Ok(())
}

pub fn public_share_requires_clean_scan(environment: &str) -> bool {
    environment != "development"
}

pub fn ensure_shareable_object_for_public_link(
    object: &ShareableObjectRecord,
    require_clean_scan: bool,
) -> Result<(), AppError> {
    if matches!(object.status, StorageObjectStatus::Quarantined) {
        return Err(AppError::conflict(
            "file_quarantined",
            "This file has been quarantined due to a policy violation.",
        ));
    }

    if !matches!(object.status, StorageObjectStatus::Active) {
        return Err(AppError::conflict(
            "object_not_shareable",
            "Only active files can be shared publicly.",
        ));
    }

    if require_clean_scan && object.scan_status != "clean" {
        return Err(AppError::conflict(
            "file_scan_not_cleared",
            "This file cannot be shared publicly until anti-malware scanning clears it.",
        ));
    }

    Ok(())
}

pub fn enforce_public_share_access(
    share: &PublicShareRecord,
    require_clean_scan: bool,
) -> Result<(), AppError> {
    if share.revoked_at.is_some() {
        return Err(AppError::forbidden(
            "share_link_revoked",
            "This public share has been revoked.",
        ));
    }

    if share.expires_at <= Utc::now() {
        return Err(AppError::forbidden(
            "share_link_expired",
            "This public share has expired.",
        ));
    }

    if matches!(share.object_status, StorageObjectStatus::Quarantined) {
        return Err(AppError::forbidden(
            "file_quarantined",
            "This file has been quarantined due to a policy violation.",
        ));
    }

    if !matches!(share.object_status, StorageObjectStatus::Active) {
        return Err(AppError::forbidden(
            "share_link_unavailable",
            "This public share is no longer available.",
        ));
    }

    if require_clean_scan && share.scan_status != "clean" {
        return Err(AppError::forbidden(
            "file_scan_not_cleared",
            "This public share is unavailable until anti-malware scanning clears the file.",
        ));
    }

    Ok(())
}

pub fn ensure_public_share_download_allowed(share: &PublicShareRecord) -> Result<(), AppError> {
    if let Some(max_downloads) = share.max_downloads
        && share.download_count >= max_downloads
    {
        return Err(AppError::forbidden(
            "share_link_download_limit_reached",
            "This share link has reached its download limit.",
        ));
    }

    Ok(())
}

pub fn build_public_share_url(_share_link_id: Uuid, token: &str) -> String {
    format!("/public/shares/{token}")
}

pub async fn build_signed_public_download_url(
    storage: &dyn nvbes_storage::ObjectStore,
    object_key: &str,
    expires_at: DateTime<Utc>,
) -> Result<SignedPublicDownloadUrlView, AppError> {
    let expires = (expires_at - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::from_secs(300));

    let presigned = storage
        .presign_download(object_key, expires)
        .await
        .map_err(|e| {
            AppError::internal("storage_error", format!("Failed to presign download: {e}"))
        })?;

    Ok(SignedPublicDownloadUrlView {
        url: presigned.url,
        method: presigned.method,
        expires_at,
    })
}

#[cfg(test)]
#[path = "drive.domains.share_links.logic.tests.rs"]
mod tests;
