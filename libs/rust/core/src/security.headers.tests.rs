use super::{
    ACCEPT_CH_VALUE, CLEAR_SITE_DATA_VALUE, CLICKJACKING_FRAME_ANCESTORS,
    CLICKJACKING_X_FRAME_OPTIONS_VALUE, CRITICAL_CH_VALUE, NEL_VALUE, PERMISSIONS_POLICY_VALUE,
    REPORT_TO_VALUE, TIMING_ALLOW_ORIGIN_VALUE, insert_cdn_cache_headers,
    insert_clear_site_data_header, insert_security_headers, no_cache_headers, security_headers,
};
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Request, Response, header},
    middleware::from_fn,
    routing::get,
};
use tower::ServiceExt;

#[test]
fn security_headers_request_all_user_agent_client_hints() {
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

    let accept_ch = headers
        .get(HeaderName::from_static("accept-ch"))
        .and_then(|value| value.to_str().ok())
        .expect("Accept-CH should be valid ASCII");
    for hint in [
        "Sec-CH-UA",
        "Sec-CH-UA-Arch",
        "Sec-CH-UA-Bitness",
        "Sec-CH-UA-Full-Version",
        "Sec-CH-UA-Full-Version-List",
        "Sec-CH-UA-Model",
        "Sec-CH-UA-WoW64",
        "Sec-CH-UA-Form-Factors",
        "Sec-CH-UA-Mobile",
        "Sec-CH-UA-Platform",
        "Sec-CH-UA-Platform-Version",
    ] {
        assert!(accept_ch.split(", ").any(|value| value == hint));
    }
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
fn security_headers_deny_clickjacking_with_csp_and_legacy_header() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(HeaderName::from_static("x-frame-options")),
        Some(&CLICKJACKING_X_FRAME_OPTIONS_VALUE)
    );

    let csp = headers
        .get(HeaderName::from_static("content-security-policy"))
        .and_then(|value| value.to_str().ok())
        .expect("content-security-policy should be present");
    assert!(csp.contains(CLICKJACKING_FRAME_ANCESTORS));
}

#[test]
fn security_headers_apply_explicit_api_csp_denylist() {
    let mut headers = HeaderMap::new();

    insert_security_headers(&mut headers);

    let csp = headers
        .get(HeaderName::from_static("content-security-policy"))
        .and_then(|value| value.to_str().ok())
        .expect("content-security-policy should be present");

    for directive in [
        "default-src 'none'",
        "script-src 'none'",
        "script-src-attr 'none'",
        "style-src 'none'",
        "img-src 'none'",
        "font-src 'none'",
        "frame-src 'none'",
        "object-src 'none'",
        "base-uri 'self'",
        "form-action 'self'",
        "report-uri /csp-report",
        "report-to nvbes-csp-endpoint",
    ] {
        assert!(
            csp.contains(directive),
            "CSP should include directive: {directive}"
        );
    }
}

#[tokio::test]
async fn security_headers_middleware_applies_clickjacking_headers_to_responses() {
    let app = Router::new()
        .route("/ok", get(|| async { "ok" }))
        .layer(from_fn(security_headers));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ok")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response
            .headers()
            .get(HeaderName::from_static("x-frame-options")),
        Some(&CLICKJACKING_X_FRAME_OPTIONS_VALUE)
    );
    assert!(
        response
            .headers()
            .get(HeaderName::from_static("content-security-policy"))
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains(CLICKJACKING_FRAME_ANCESTORS))
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

#[tokio::test]
async fn no_cache_headers_preserve_explicit_cache_control() {
    let app = Router::new()
        .route(
            "/cacheable",
            get(|| async {
                Response::builder()
                    .header(header::CACHE_CONTROL, "public, max-age=60")
                    .body(Body::empty())
                    .expect("response should build")
            }),
        )
        .layer(from_fn(no_cache_headers));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/cacheable")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&HeaderValue::from_static("public, max-age=60"))
    );
    assert!(response.headers().get(header::PRAGMA).is_none());
}

#[tokio::test]
async fn no_cache_headers_apply_when_cache_control_absent() {
    let app = Router::new()
        .route("/ok", get(|| async { "ok" }))
        .layer(from_fn(no_cache_headers));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ok")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&HeaderValue::from_static(
            "no-store, no-cache, must-revalidate"
        ))
    );
    assert_eq!(
        response.headers().get(header::PRAGMA),
        Some(&HeaderValue::from_static("no-cache"))
    );
}

#[test]
fn security_headers_preserve_wildcard_vary() {
    let mut headers = HeaderMap::new();
    headers.insert(header::VARY, HeaderValue::from_static("*"));

    insert_security_headers(&mut headers);

    assert_eq!(
        headers.get(header::VARY),
        Some(&HeaderValue::from_static("*"))
    );
}

#[test]
fn security_headers_strip_stack_fingerprint_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-powered-by"),
        HeaderValue::from_static("Express"),
    );
    headers.insert(
        HeaderName::from_static("server"),
        HeaderValue::from_static("nginx"),
    );
    headers.insert(
        HeaderName::from_static("x-aspnet-version"),
        HeaderValue::from_static("4.0"),
    );
    headers.insert(
        HeaderName::from_static("x-aspnetmvc-version"),
        HeaderValue::from_static("5.2"),
    );

    insert_security_headers(&mut headers);

    for name in [
        "x-powered-by",
        "server",
        "x-aspnet-version",
        "x-aspnetmvc-version",
    ] {
        assert!(
            headers.get(HeaderName::from_static(name)).is_none(),
            "{name} should be stripped"
        );
    }
}

#[test]
fn insert_cdn_cache_headers_table_covers_swr_branches() {
    let cases = [
        (60, 0, "public, s-maxage=60"),
        (120, 30, "public, s-maxage=120, stale-while-revalidate=30"),
    ];
    for (max_age, swr, expected) in cases {
        let mut headers = HeaderMap::new();
        insert_cdn_cache_headers(&mut headers, max_age, swr);
        assert_eq!(
            headers
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok()),
            Some(expected)
        );
        assert_eq!(
            headers
                .get(HeaderName::from_static("cdn-cache-control"))
                .and_then(|v| v.to_str().ok()),
            Some(expected)
        );
        assert_eq!(
            headers
                .get(HeaderName::from_static("surrogate-control"))
                .and_then(|v| v.to_str().ok()),
            Some(expected)
        );
    }
}

#[test]
fn security_headers_append_csp_report_to_group() {
    let mut headers = HeaderMap::new();
    insert_security_headers(&mut headers);

    let report_to: Vec<_> = headers
        .get_all(HeaderName::from_static("report-to"))
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    assert!(
        report_to.iter().any(|v| v.contains("nvbes-network-errors")),
        "{report_to:?}"
    );
    assert!(
        report_to.iter().any(|v| v.contains("nvbes-csp-endpoint")),
        "{report_to:?}"
    );
}
