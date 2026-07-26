use axum::{
    extract::State,
    http::{HeaderMap, Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::app::AppState;
use crate::http::error::AppError;

use super::PUBLIC_REPORT_PATHS;

const CSRF_SKIP_PATHS: &[&str] = &[
    "/auth/register",
    "/auth/verify-email",
    "/auth/verify-email/resend",
    "/auth/verify-email/change",
    "/auth/challenge/pow",
    "/auth/challenge/identifier",
    "/auth/challenge/pwd",
    "/auth/challenge/mfa",
    "/auth/challenge/mfa/email/send",
    "/auth/challenge/webauthn/start",
    "/auth/challenge/webauthn/discoverable/start",
    "/auth/challenge/webauthn/discoverable/finish",
    "/auth/password/forgot",
    "/auth/password/reset",
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

    if PUBLIC_REPORT_PATHS.contains(&request.uri().path()) {
        return Ok(next.run(request).await);
    }

    if CSRF_SKIP_PATHS.contains(&request.uri().path()) {
        return Ok(next.run(request).await);
    }

    let authuser = crate::http::authuser::from_uri_and_headers(request.uri(), &headers)?;

    let secure_cookie = state.config.environment != "development";
    let session_cookie_base =
        crate::http::cookies::auth_cookie_name_with_user("session", &authuser, secure_cookie);

    let Some(session_cookie_value) = extract_cookie_value(&headers, &session_cookie_base) else {
        return Ok(next.run(request).await);
    };

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

    validate_signed_double_submit_csrf(
        &cookie_value,
        header_value,
        &session_cookie_value,
        &state.config.jwt_secret,
    )?;

    Ok(next.run(request).await)
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    )
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

fn validate_signed_double_submit_csrf(
    cookie_value: &str,
    header_value: &str,
    session_cookie_value: &str,
    secret: &str,
) -> Result<(), AppError> {
    if cookie_value != header_value {
        return Err(AppError::forbidden(
            "csrf_token_mismatch",
            "CSRF token does not match.",
        ));
    }

    if crate::http::cookies::verify_csrf_token(header_value, session_cookie_value, secret) {
        return Ok(());
    }

    Err(AppError::forbidden(
        "csrf_token_invalid",
        "CSRF token is not valid for this session.",
    ))
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

    use crate::http::cookies::generate_csrf_token;

    #[test]
    fn fetch_metadata_rejects_cross_site_requests() {
        let mut headers = HeaderMap::new();
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("cross-site"));

        let error = super::reject_cross_site_fetch_metadata(&headers)
            .expect_err("cross-site fetch metadata should be rejected");

        assert_eq!(error.code, "cross_site_request");
    }

    #[test]
    fn public_registration_and_verification_do_not_require_session_csrf() {
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/register"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/verify-email"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/verify-email/resend"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/verify-email/change"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/pow"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/identifier"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/pwd"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/mfa"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/mfa/email/send"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/webauthn/start"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/webauthn/discoverable/start"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/challenge/webauthn/discoverable/finish"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/password/forgot"));
        assert!(super::CSRF_SKIP_PATHS.contains(&"/auth/password/reset"));
    }

    #[test]
    fn browser_reports_do_not_require_session_csrf() {
        assert!(super::PUBLIC_REPORT_PATHS.contains(&"/csp-report"));
        assert!(super::PUBLIC_REPORT_PATHS.contains(&"/observability/network-errors"));
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

    #[test]
    fn signed_double_submit_accepts_token_bound_to_session() {
        let token = generate_csrf_token("session-a", "secret");

        assert!(
            super::validate_signed_double_submit_csrf(&token, &token, "session-a", "secret")
                .is_ok()
        );
    }

    #[test]
    fn signed_double_submit_rejects_header_cookie_mismatch() {
        let token = generate_csrf_token("session-a", "secret");
        let other = generate_csrf_token("session-a", "secret");

        let error =
            super::validate_signed_double_submit_csrf(&token, &other, "session-a", "secret")
                .expect_err("mismatched CSRF token should be rejected");

        assert_eq!(error.code, "csrf_token_mismatch");
    }

    #[test]
    fn signed_double_submit_rejects_token_from_other_session() {
        let token = generate_csrf_token("session-a", "secret");

        let error =
            super::validate_signed_double_submit_csrf(&token, &token, "session-b", "secret")
                .expect_err("CSRF token from another session should be rejected");

        assert_eq!(error.code, "csrf_token_invalid");
    }

    #[test]
    fn signed_double_submit_rejects_unsigned_legacy_token() {
        let error = super::validate_signed_double_submit_csrf(
            "plain-random",
            "plain-random",
            "session-a",
            "secret",
        )
        .expect_err("unsigned CSRF token should be rejected");

        assert_eq!(error.code, "csrf_token_invalid");
    }
}
