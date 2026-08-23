use chrono::{Duration as ChronoDuration, Utc};
use uuid::Uuid;

use super::*;
use crate::domains::{
    files::models::StorageObjectStatus, share_links::models::ShareLinkPermission,
};

fn policy() -> SharePolicy {
    SharePolicy {
        default_ttl_days: 7,
        max_ttl_days: 30,
        max_share_links: 100,
    }
}

fn shareable_object(status: StorageObjectStatus, scan_status: &str) -> ShareableObjectRecord {
    ShareableObjectRecord {
        id: Uuid::new_v4(),
        status,
        scan_status: scan_status.to_owned(),
    }
}

fn share_link(expires_offset: ChronoDuration, revoked: bool) -> ShareLinkRecord {
    let now = Utc::now();
    ShareLinkRecord {
        id: Uuid::new_v4(),
        workspace_id: Uuid::new_v4(),
        storage_object_id: Uuid::new_v4(),
        permission: ShareLinkPermission::Download,
        expires_at: now + expires_offset,
        max_downloads: Some(3),
        download_count: 0,
        created_by: Uuid::new_v4(),
        created_by_principal_id: Uuid::new_v4(),
        revoked_at: revoked.then_some(now),
        created_at: now,
        updated_at: now,
    }
}

fn public_share() -> PublicShareRecord {
    PublicShareRecord {
        share_link_id: Uuid::new_v4(),
        workspace_id: Uuid::new_v4(),
        storage_object_id: Uuid::new_v4(),
        expires_at: Utc::now() + ChronoDuration::hours(1),
        max_downloads: Some(3),
        download_count: 2,
        revoked_at: None,
        name: "report.pdf".to_owned(),
        mime_type: Some("application/pdf".to_owned()),
        size_bytes: 512,
        object_status: StorageObjectStatus::Active,
        scan_status: "clean".to_owned(),
        object_key: Some("workspaces/ws/objects/object".to_owned()),
    }
}

#[test]
fn normalize_expires_at_rejects_past_and_ttl_overflow() {
    let past = normalize_expires_at(Some(Utc::now() - ChronoDuration::seconds(1)), &policy())
        .expect_err("past expiration should be rejected");
    assert_eq!(past.code, "validation_failed");

    let too_far = normalize_expires_at(Some(Utc::now() + ChronoDuration::days(31)), &policy())
        .expect_err("expiration beyond policy max should be rejected");
    assert_eq!(too_far.code, "validation_failed");
}

#[test]
fn normalize_max_downloads_rejects_zero_or_negative_limits() {
    assert_eq!(
        normalize_max_downloads(Some(0))
            .expect_err("zero should be rejected")
            .code,
        "validation_failed"
    );
    assert_eq!(
        normalize_max_downloads(Some(-1))
            .expect_err("negative should be rejected")
            .code,
        "validation_failed"
    );
    assert_eq!(normalize_max_downloads(Some(1)).unwrap(), Some(1));
}

#[test]
fn ensure_link_not_revoked_or_expired_blocks_mutation() {
    assert_eq!(
        ensure_link_not_revoked_or_expired(&share_link(ChronoDuration::hours(1), true))
            .expect_err("revoked link should reject update")
            .code,
        "share_link_revoked"
    );
    assert_eq!(
        ensure_link_not_revoked_or_expired(&share_link(ChronoDuration::hours(-1), false))
            .expect_err("expired link should reject update")
            .code,
        "share_link_expired"
    );
}

#[test]
fn enforce_public_share_access_rejects_revoked_expired_and_unclean_links() {
    let mut share = public_share();
    share.revoked_at = Some(Utc::now());
    assert_eq!(
        enforce_public_share_access(&share, false)
            .expect_err("revoked public share should be forbidden")
            .code,
        "share_link_revoked"
    );

    share = public_share();
    share.expires_at = Utc::now() - ChronoDuration::seconds(1);
    assert_eq!(
        enforce_public_share_access(&share, false)
            .expect_err("expired public share should be forbidden")
            .code,
        "share_link_expired"
    );

    share = public_share();
    share.scan_status = "pending".to_owned();
    assert_eq!(
        enforce_public_share_access(&share, true)
            .expect_err("unclean public share should be forbidden")
            .code,
        "file_scan_not_cleared"
    );
}

#[test]
fn ensure_public_share_download_allowed_blocks_at_limit_only() {
    let mut share = public_share();
    ensure_public_share_download_allowed(&share).expect("below max downloads should be allowed");

    share.download_count = 3;
    assert_eq!(
        ensure_public_share_download_allowed(&share)
            .expect_err("limit should block download URL creation")
            .code,
        "share_link_download_limit_reached"
    );

    share.max_downloads = None;
    ensure_public_share_download_allowed(&share).expect("unlimited shares should be allowed");
}

#[test]
fn public_share_requires_clean_scan_outside_development() {
    assert!(public_share_requires_clean_scan("staging"));
    assert!(public_share_requires_clean_scan("production"));
    assert!(!public_share_requires_clean_scan("development"));
}

#[test]
fn active_but_unclean_object_is_not_publicly_shareable_when_scan_is_required() {
    let object = shareable_object(StorageObjectStatus::Active, "unscanned_disabled");

    let error = ensure_shareable_object_for_public_link(&object, true)
        .expect_err("unclean scan should block public sharing");

    assert_eq!(error.code, "file_scan_not_cleared");
}

#[test]
fn development_can_share_active_unscanned_object() {
    let object = shareable_object(StorageObjectStatus::Active, "unscanned_disabled");

    ensure_shareable_object_for_public_link(&object, false)
        .expect("development should allow active unscanned objects");
}

#[test]
fn quarantined_object_is_never_publicly_shareable() {
    let object = shareable_object(StorageObjectStatus::Quarantined, "infected");

    let error = ensure_shareable_object_for_public_link(&object, false)
        .expect_err("quarantine should always block public sharing");

    assert_eq!(error.code, "file_quarantined");
}
