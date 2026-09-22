use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

use super::{
    ensure_if_match, etag_list_matches, if_match, insert_etag, not_modified_if_none_match,
    resource_etag,
};

#[test]
fn resource_etag_is_stable_for_same_inputs() {
    let id = Uuid::nil();
    let updated_at = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap();
    let etag = resource_etag("user", id, updated_at);
    assert!(etag.starts_with("\"user:"));
    assert!(etag.ends_with('"'));
}

#[test]
fn insert_etag_adds_header_value() {
    let mut headers = HeaderMap::new();
    insert_etag(&mut headers, "\"user:1\"").expect("valid etag");
    assert_eq!(
        headers.get(header::ETAG).and_then(|v| v.to_str().ok()),
        Some("\"user:1\"")
    );
}

#[test]
fn ensure_if_match_accepts_matching_or_missing_header() {
    let mut headers = HeaderMap::new();
    ensure_if_match(&headers, "\"v1\"").expect("missing header allows update");
    headers.insert(header::IF_MATCH, HeaderValue::from_static("\"v1\", \"v0\""));
    ensure_if_match(&headers, "\"v1\"").expect("list match");
    headers.insert(header::IF_MATCH, HeaderValue::from_static("\"v2\""));
    assert!(ensure_if_match(&headers, "\"v1\"").is_err());
}

#[test]
fn not_modified_if_none_match_detects_wildcard_and_values() {
    let mut headers = HeaderMap::new();
    assert!(
        not_modified_if_none_match(&headers, "\"v1\"")
            .unwrap()
            .is_none()
    );
    headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));
    assert_eq!(
        not_modified_if_none_match(&headers, "\"v1\"").unwrap(),
        Some(StatusCode::NOT_MODIFIED)
    );
}

#[test]
fn etag_list_matches_supports_wildcard_and_lists() {
    assert!(etag_list_matches("*", "\"v1\""));
    assert!(etag_list_matches("\"v1\", \"v2\"", "\"v2\""));
    assert!(!etag_list_matches("\"v3\"", "\"v1\""));
}

#[test]
fn if_match_rejects_invalid_header_encoding() {
    let mut headers = HeaderMap::new();
    headers.insert(header::IF_MATCH, HeaderValue::from_bytes(&[0xff]).unwrap());
    assert!(if_match(&headers).is_err());
}
