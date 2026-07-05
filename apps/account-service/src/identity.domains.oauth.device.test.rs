use super::*;
use crate::domains::auth::RateLimiter;
use crate::domains::oauth::device_validation::{
    enforce_device_action_rate_limit, enforce_device_authorization_rate_limit,
};
use axum::http::{HeaderName, HeaderValue};

fn headers_with_ip(ip: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static(nvbes_core::http::client_ip::TRUSTED_CLIENT_IP_HEADER),
        HeaderValue::from_str(ip).expect("valid ip header"),
    );
    headers
}

#[test]
fn device_authorization_rate_limits_by_ip() {
    let limiter = RateLimiter::new();
    let headers = headers_with_ip("203.0.113.10");

    for _ in 0..12 {
        enforce_device_authorization_rate_limit(&limiter, &headers, "client_a")
            .expect("request should pass");
    }

    let error = enforce_device_authorization_rate_limit(&limiter, &headers, "client_a")
        .expect_err("request should be rate limited");

    assert_eq!(error.code, "rate_limited");
}

#[test]
fn device_authorization_rate_limits_by_client_id() {
    let limiter = RateLimiter::new();

    for idx in 0..24 {
        let headers = headers_with_ip(&format!("203.0.113.{}", idx + 1));
        enforce_device_authorization_rate_limit(&limiter, &headers, "client_a")
            .expect("request should pass");
    }

    let headers = headers_with_ip("203.0.113.250");
    let error = enforce_device_authorization_rate_limit(&limiter, &headers, "client_a")
        .expect_err("request should be rate limited");

    assert_eq!(error.code, "rate_limited");
}

#[test]
fn device_action_rate_limits_by_user_id() {
    let limiter = RateLimiter::new();
    let user_id = Uuid::new_v4();

    for idx in 0..12 {
        enforce_device_action_rate_limit(
            &limiter,
            "oauth_device_approve",
            user_id,
            &format!("GX-{}", idx),
        )
        .expect("request should pass");
    }

    let error =
        enforce_device_action_rate_limit(&limiter, "oauth_device_approve", user_id, "GX-9999")
            .expect_err("request should be rate limited");

    assert_eq!(error.code, "rate_limited");
}

#[test]
fn device_action_rate_limits_by_user_code() {
    let limiter = RateLimiter::new();

    for _ in 0..6 {
        enforce_device_action_rate_limit(&limiter, "oauth_device_deny", Uuid::new_v4(), "GX-4321")
            .expect("request should pass");
    }

    let error =
        enforce_device_action_rate_limit(&limiter, "oauth_device_deny", Uuid::new_v4(), "GX-4321")
            .expect_err("request should be rate limited");

    assert_eq!(error.code, "rate_limited");
}
