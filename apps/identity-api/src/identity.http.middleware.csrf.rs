use axum::{
    extract::State,
    http::{HeaderMap, Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::app::AppState;
use crate::http::error::AppError;

const CSRF_SKIP_PATHS: &[&str] = &[
    "/oauth/token",
    "/oauth/introspect",
    "/oauth/revoke",
    "/oauth/par",
];

pub async fn csrf_guard(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    if !is_mutating_method(request.method()) {
        return Ok(next.run(request).await);
    }

    if CSRF_SKIP_PATHS.contains(&request.uri().path()) {
        return Ok(next.run(request).await);
    }

    let authuser = request
        .uri()
        .query()
        .and_then(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "authuser")
                .map(|(_, v)| v.into_owned())
        })
        .or_else(|| {
            headers
                .get("X-Auth-User")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "0".to_string());

    let secure_cookie = state.config.environment != "development";
    let session_cookie_base =
        crate::http::cookies::auth_cookie_name_with_user("session", &authuser, secure_cookie);

    if !has_cookie(&headers, &session_cookie_base) {
        return Ok(next.run(request).await);
    }

    reject_cross_site_fetch_metadata(&headers)?;

    let csrf_cookie_base =
        crate::http::cookies::auth_cookie_name_with_user("csrf_token", &authuser, secure_cookie);
    let cookie_value = extract_cookie_value(&headers, &csrf_cookie_base)
        .ok_or_else(|| AppError::forbidden("missing_csrf_token", "CSRF token cookie not found."))?;

    let header_value = headers
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::forbidden("missing_csrf_header", "X-CSRF-Token header is required.")
        })?;

    if cookie_value != header_value {
        return Err(AppError::forbidden(
            "csrf_token_mismatch",
            "CSRF token does not match.",
        ));
    }

    Ok(next.run(request).await)
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    )
}

fn has_cookie(headers: &HeaderMap, name: &str) -> bool {
    let prefix = format!("{name}=");
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|cookie_str| cookie_str.split(';').any(|c| c.trim().starts_with(&prefix)))
}

fn reject_cross_site_fetch_metadata(headers: &HeaderMap) -> Result<(), AppError> {
    reject_cross_site_request(headers)?;
    reject_untrusted_request_mode(headers)?;
    reject_untrusted_request_destination(headers)?;

    Ok(())
}

fn reject_cross_site_request(headers: &HeaderMap) -> Result<(), AppError> {
    let Some(site) = header_str(headers, "Sec-Fetch-Site") else {
        return Ok(());
    };

    if site.eq_ignore_ascii_case("cross-site") {
        return Err(AppError::forbidden(
            "cross_site_request",
            "Cross-site authenticated requests are not allowed.",
        ));
    }

    Ok(())
}

fn reject_untrusted_request_mode(headers: &HeaderMap) -> Result<(), AppError> {
    let Some(mode) = header_str(headers, "Sec-Fetch-Mode") else {
        return Ok(());
    };

    if matches_ignore_ascii_case(mode, &["cors", "same-origin", "navigate"]) {
        return Ok(());
    }

    Err(AppError::forbidden(
        "untrusted_fetch_mode",
        "Authenticated request mode is not allowed.",
    ))
}

fn reject_untrusted_request_destination(headers: &HeaderMap) -> Result<(), AppError> {
    let Some(dest) = header_str(headers, "Sec-Fetch-Dest") else {
        return Ok(());
    };

    if matches_ignore_ascii_case(dest, &["empty", "document"]) {
        return Ok(());
    }

    Err(AppError::forbidden(
        "untrusted_fetch_destination",
        "Authenticated request destination is not allowed.",
    ))
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn matches_ignore_ascii_case(value: &str, allowed: &[&str]) -> bool {
    allowed
        .iter()
        .any(|allowed_value| value.eq_ignore_ascii_case(allowed_value))
}

fn extract_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    let cookie_str = headers.get(header::COOKIE)?.to_str().ok()?;

    cookie_str.split(';').find_map(|cookie| {
        let trimmed = cookie.trim();
        trimmed.strip_prefix(&prefix).map(|v| v.to_string())
    })
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn fetch_metadata_rejects_cross_site_requests() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("cross-site"));

        let error = super::reject_cross_site_fetch_metadata(&headers)
            .expect_err("cross-site fetch metadata should be rejected");

        assert_eq!(error.code, "cross_site_request");
    }

    #[test]
    fn fetch_metadata_accepts_same_origin_requests() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));

        assert!(super::reject_cross_site_fetch_metadata(&headers).is_ok());
    }

    #[test]
    fn fetch_metadata_accepts_same_site_cors_api_requests() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-site"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));

        assert!(super::reject_cross_site_fetch_metadata(&headers).is_ok());
    }

    #[test]
    fn fetch_metadata_accepts_browser_form_posts() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));

        assert!(super::reject_cross_site_fetch_metadata(&headers).is_ok());
    }

    #[test]
    fn fetch_metadata_rejects_no_cors_authenticated_requests() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("no-cors"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));

        let error = super::reject_cross_site_fetch_metadata(&headers)
            .expect_err("no-cors fetch metadata should be rejected");

        assert_eq!(error.code, "untrusted_fetch_mode");
    }

    #[test]
    fn fetch_metadata_rejects_subresource_destinations() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("image"));

        let error = super::reject_cross_site_fetch_metadata(&headers)
            .expect_err("subresource destinations should be rejected");

        assert_eq!(error.code, "untrusted_fetch_destination");
    }

    #[test]
    fn fetch_metadata_accepts_requests_without_browser_headers() {
        let headers = HeaderMap::new();

        assert!(super::reject_cross_site_fetch_metadata(&headers).is_ok());
    }
}
