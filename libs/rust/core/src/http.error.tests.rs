use axum::body::to_bytes;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::limiter::RateLimitInfo;

use super::{AppError, ErrorRecovery};

#[tokio::test]
async fn into_response_maps_internal_errors_to_public_contract() {
    let response = AppError::internal("database_error", "constraint failed").into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("internal_error"));
    assert!(json.contains("Internal server error."));
    assert!(!json.contains("constraint"));
}

#[tokio::test]
async fn into_response_preserves_reauthentication_recovery() {
    let response = AppError::reauthenticate("session_expired", "Session expired.").into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("reauthenticate"));
}

#[tokio::test]
async fn into_response_attaches_rate_limit_headers() {
    let info = RateLimitInfo {
        limit: 10,
        remaining: 0,
        reset: 42,
    };
    let response = AppError::too_many_requests("rate_limited", "Slow down.", Some(30), Some(info))
        .into_response();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok()),
        Some("30")
    );
    assert_eq!(
        response
            .headers()
            .get("ratelimit-limit")
            .and_then(|v| v.to_str().ok()),
        Some("10")
    );
    assert_eq!(
        response
            .headers()
            .get("ratelimit-remaining")
            .and_then(|v| v.to_str().ok()),
        Some("0")
    );
}

#[test]
fn status_specific_constructors_set_expected_codes() {
    let cases = [
        (
            AppError::forbidden("denied", "Denied."),
            StatusCode::FORBIDDEN,
        ),
        (
            AppError::not_found("missing", "Missing."),
            StatusCode::NOT_FOUND,
        ),
        (
            AppError::conflict("duplicate", "Duplicate."),
            StatusCode::CONFLICT,
        ),
        (
            AppError::precondition_failed("stale", "Stale."),
            StatusCode::PRECONDITION_FAILED,
        ),
    ];
    for (error, status) in cases {
        assert_eq!(error.status, status);
    }
}

#[test]
fn requiring_reauthentication_sets_recovery_flag() {
    let error = AppError::unauthorized("expired", "Expired.").requiring_reauthentication();
    assert_eq!(error.recovery, Some(ErrorRecovery::Reauthenticate));
}
