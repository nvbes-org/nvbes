use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, header},
    middleware::Next,
    response::Response,
};

const ACCEPT_CH_VALUE: HeaderValue = HeaderValue::from_static(
    "Sec-CH-UA, Sec-CH-UA-Arch, Sec-CH-UA-Bitness, Sec-CH-UA-Full-Version, Sec-CH-UA-Full-Version-List, Sec-CH-UA-Model, Sec-CH-UA-WoW64, Sec-CH-UA-Form-Factors, Sec-CH-UA-Mobile, Sec-CH-UA-Platform, Sec-CH-UA-Platform-Version",
);
const CLEAR_SITE_DATA_VALUE: HeaderValue =
    HeaderValue::from_static("\"cache\", \"cookies\", \"storage\", \"executionContexts\"");
const CRITICAL_CH_VALUE: HeaderValue =
    HeaderValue::from_static("Sec-CH-UA, Sec-CH-UA-Platform, Sec-CH-UA-Mobile");
#[cfg(test)]
const CLICKJACKING_FRAME_ANCESTORS: &str = "frame-ancestors 'none'";
const CLICKJACKING_X_FRAME_OPTIONS_VALUE: HeaderValue = HeaderValue::from_static("DENY");
const DOCUMENT_POLICY_VALUE: HeaderValue =
    HeaderValue::from_static("oversized-images=2.0, unsized-media=2.0, force-load-at-top");
const EXPECT_CT_VALUE: HeaderValue = HeaderValue::from_static("max-age=86400, enforce");
const NEL_VALUE: HeaderValue = HeaderValue::from_static(
    r#"{"report_to":"nvbes-network-errors","max_age":10886400,"include_subdomains":true,"success_fraction":0.0,"failure_fraction":1.0}"#,
);
const PERMISSIONS_POLICY_VALUE: HeaderValue = HeaderValue::from_static(
    "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=(), interest-cohort=()",
);
const REPORT_TO_VALUE: HeaderValue = HeaderValue::from_static(
    r#"{"group":"nvbes-network-errors","max_age":10886400,"endpoints":[{"url":"/observability/network-errors"}],"include_subdomains":true}"#,
);
const CSP_REPORT_TO_VALUE: HeaderValue = HeaderValue::from_static(
    r#"{"group":"nvbes-csp-endpoint","max_age":10886400,"endpoints":[{"url":"/csp-report"}],"include_subdomains":true}"#,
);
const CSP_VALUE: HeaderValue = HeaderValue::from_static(
    "default-src 'none'; script-src 'none'; script-src-attr 'none'; style-src 'none'; img-src 'none'; font-src 'none'; connect-src 'self'; worker-src 'none'; child-src 'none'; frame-src 'none'; object-src 'none'; manifest-src 'none'; media-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; report-uri /csp-report; report-to nvbes-csp-endpoint",
);
const REQUIRED_VARY_HEADERS: [&str; 2] = ["Accept-Encoding", "Authorization"];
const TIMING_ALLOW_ORIGIN_VALUE: HeaderValue = HeaderValue::from_static("*");

pub async fn security_headers(
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    insert_security_headers(response.headers_mut());

    response
}

fn insert_security_headers(headers: &mut HeaderMap) {
    // Strip stack-fingerprinting headers first (before adding ours).
    headers.remove(HeaderName::from_static("x-powered-by"));
    headers.remove(HeaderName::from_static("server"));
    headers.remove(HeaderName::from_static("x-aspnet-version"));
    headers.remove(HeaderName::from_static("x-aspnetmvc-version"));

    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(HeaderName::from_static("expect-ct"), EXPECT_CT_VALUE);
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        CLICKJACKING_X_FRAME_OPTIONS_VALUE,
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        CSP_VALUE,
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        PERMISSIONS_POLICY_VALUE,
    );
    headers.insert(
        HeaderName::from_static("document-policy"),
        DOCUMENT_POLICY_VALUE,
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("origin-agent-cluster"),
        HeaderValue::from_static("?1"),
    );
    headers.insert(
        HeaderName::from_static("x-permitted-cross-domain-policies"),
        HeaderValue::from_static("none"),
    );
    headers.insert(
        HeaderName::from_static("x-download-options"),
        HeaderValue::from_static("noopen"),
    );
    headers.insert(HeaderName::from_static("accept-ch"), ACCEPT_CH_VALUE);
    headers.insert(HeaderName::from_static("critical-ch"), CRITICAL_CH_VALUE);
    headers.insert(
        HeaderName::from_static("timing-allow-origin"),
        TIMING_ALLOW_ORIGIN_VALUE,
    );
    headers.insert(HeaderName::from_static("report-to"), REPORT_TO_VALUE);
    headers.append(
        HeaderName::from_static("report-to"),
        CSP_REPORT_TO_VALUE.clone(),
    );
    headers.insert(HeaderName::from_static("nel"), NEL_VALUE);
    ensure_required_vary_headers(headers);
}

fn ensure_required_vary_headers(headers: &mut HeaderMap) {
    let Some(existing) = headers
        .get(header::VARY)
        .and_then(|value| value.to_str().ok())
    else {
        headers.insert(
            header::VARY,
            HeaderValue::from_static("Accept-Encoding, Authorization"),
        );
        return;
    };

    if existing.trim() == "*" {
        return;
    }

    let mut values = existing.to_owned();
    for required in REQUIRED_VARY_HEADERS {
        if !existing
            .split(',')
            .any(|value| value.trim().eq_ignore_ascii_case(required))
        {
            values.push_str(", ");
            values.push_str(required);
        }
    }

    if let Ok(value) = HeaderValue::from_str(&values) {
        headers.insert(header::VARY, value);
    }
}

pub fn insert_clear_site_data_header(headers: &mut HeaderMap) {
    headers.insert(
        HeaderName::from_static("clear-site-data"),
        CLEAR_SITE_DATA_VALUE,
    );
}

pub fn insert_cdn_cache_headers(headers: &mut HeaderMap, max_age_secs: u64, swr_secs: u64) {
    let value = if swr_secs > 0 {
        format!(
            "public, s-maxage={}, stale-while-revalidate={}",
            max_age_secs, swr_secs
        )
    } else {
        format!("public, s-maxage={}", max_age_secs)
    };
    if let Ok(v) = HeaderValue::from_str(&value) {
        headers.insert(header::CACHE_CONTROL, v.clone());
        headers.insert(HeaderName::from_static("cdn-cache-control"), v.clone());
        headers.insert(HeaderName::from_static("surrogate-control"), v);
    }
}

pub async fn no_cache_headers(
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    if !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate"),
        );
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }
    response
}

#[cfg(test)]
#[path = "security.headers.tests.rs"]
mod tests;
