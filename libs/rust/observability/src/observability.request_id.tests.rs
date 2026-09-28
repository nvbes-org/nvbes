use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Request};

use super::{
    REQUEST_ID_HEADER, inbound_request_id, is_safe_request_id, new_request_id,
    request_id_from_headers, request_id_header,
};

#[test]
fn generated_request_ids_use_safe_prefix() {
    let request_id = new_request_id();
    assert!(request_id.starts_with("req_"));
    assert!(is_safe_request_id(&request_id));
}

#[test]
fn unsafe_request_ids_are_rejected() {
    assert!(!is_safe_request_id(""));
    assert!(!is_safe_request_id("req with spaces"));
    assert!(!is_safe_request_id("req/with/slashes"));
    assert!(!is_safe_request_id("req@invalid"));
    assert!(!is_safe_request_id(&"a".repeat(97)));
}

#[test]
fn safe_request_ids_accept_allowed_charset() {
    assert!(is_safe_request_id("req_abc-DEF.123"));
    assert!(is_safe_request_id(&"a".repeat(96)));
}

#[test]
fn request_id_header_is_stable() {
    assert_eq!(request_id_header().as_str(), REQUEST_ID_HEADER);
}

#[test]
fn request_id_from_headers_reuses_safe_inbound_value() {
    let mut headers = HeaderMap::new();
    headers.insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_static("req_inbound-value"),
    );
    assert_eq!(request_id_from_headers(&headers), "req_inbound-value");
}

#[test]
fn request_id_from_headers_generates_when_missing_or_unsafe() {
    let empty = HeaderMap::new();
    let generated = request_id_from_headers(&empty);
    assert!(generated.starts_with("req_"));

    let mut headers = HeaderMap::new();
    headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static("bad id"));
    let replaced = request_id_from_headers(&headers);
    assert!(replaced.starts_with("req_"));
    assert_ne!(replaced, "bad id");
}

#[test]
fn request_id_from_headers_trims_whitespace() {
    let mut headers = HeaderMap::new();
    headers.insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_static("  req_trimmed  "),
    );
    assert_eq!(request_id_from_headers(&headers), "req_trimmed");
}

#[test]
fn inbound_request_id_returns_none_without_header() {
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    assert!(inbound_request_id(&req).is_none());
}

#[test]
fn inbound_request_id_returns_safe_header() {
    let req = Request::builder()
        .uri("/")
        .header(REQUEST_ID_HEADER, "req_from_request")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        inbound_request_id(&req).as_deref(),
        Some("req_from_request")
    );
}
