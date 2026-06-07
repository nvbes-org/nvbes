use super::{
    ACCEPT_CH_VALUE, CLEAR_SITE_DATA_VALUE, CRITICAL_CH_VALUE, NEL_VALUE, PERMISSIONS_POLICY_VALUE,
    REPORT_TO_VALUE, TIMING_ALLOW_ORIGIN_VALUE, insert_clear_site_data_header,
    insert_security_headers,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, header};

#[test]
fn security_headers_requests_low_entropy_client_hints() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(HeaderName::from_static("accept-ch")),
        Some(&ACCEPT_CH_VALUE)
    );
    assert_eq!(
        headers.get(HeaderName::from_static("critical-ch")),
        Some(&CRITICAL_CH_VALUE)
    );
}

#[test]
fn security_headers_expose_browser_timing_observability() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(HeaderName::from_static("timing-allow-origin")),
        Some(&TIMING_ALLOW_ORIGIN_VALUE)
    );
    assert_eq!(
        headers.get(HeaderName::from_static("report-to")),
        Some(&REPORT_TO_VALUE)
    );
    assert_eq!(
        headers.get(HeaderName::from_static("nel")),
        Some(&NEL_VALUE)
    );
}

#[test]
fn security_headers_disable_browser_capability_apis() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(HeaderName::from_static("permissions-policy")),
        Some(&PERMISSIONS_POLICY_VALUE)
    );
}

#[test]
fn security_headers_enable_browser_isolation() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(HeaderName::from_static("cross-origin-opener-policy")),
        Some(&HeaderValue::from_static("same-origin"))
    );
    assert_eq!(
        headers.get(HeaderName::from_static("cross-origin-embedder-policy")),
        Some(&HeaderValue::from_static("require-corp"))
    );
    assert_eq!(
        headers.get(HeaderName::from_static("origin-agent-cluster")),
        Some(&HeaderValue::from_static("?1"))
    );
}

#[test]
fn clear_site_data_header_clears_browser_state() {
    let mut headers = HeaderMap::new();

    insert_clear_site_data_header(&mut headers);

    assert_eq!(
        headers.get(HeaderName::from_static("clear-site-data")),
        Some(&CLEAR_SITE_DATA_VALUE)
    );
}

#[test]
fn security_headers_vary_by_encoding_and_authorization() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(header::VARY),
        Some(&HeaderValue::from_static("Accept-Encoding, Authorization"))
    );
}

#[test]
fn security_headers_preserve_existing_vary_values() {
    let mut headers = HeaderMap::new();
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(header::VARY),
        Some(&HeaderValue::from_static(
            "Origin, Accept-Encoding, Authorization"
        ))
    );
}

#[test]
fn security_headers_do_not_duplicate_vary_values() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::VARY,
        HeaderValue::from_static("origin, accept-encoding, authorization"),
    );

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(header::VARY),
        Some(&HeaderValue::from_static(
            "origin, accept-encoding, authorization"
        ))
    );
}
