use axum::response::IntoResponse;
use sqlx::Error as SqlxError;

use super::AccountError;

#[test]
fn account_errors_map_to_stable_http_contracts() {
    let cases = [
        (
            AccountError::Unauthorized,
            axum::http::StatusCode::UNAUTHORIZED,
            "authentication required",
        ),
        (
            AccountError::Forbidden,
            axum::http::StatusCode::FORBIDDEN,
            "insufficient scope",
        ),
        (
            AccountError::NotFound,
            axum::http::StatusCode::NOT_FOUND,
            "resource not found",
        ),
        (
            AccountError::Conflict,
            axum::http::StatusCode::CONFLICT,
            "request conflicts with current state",
        ),
        (
            AccountError::Invalid("team name length"),
            axum::http::StatusCode::BAD_REQUEST,
            "invalid request: team name length",
        ),
        (
            AccountError::Database(SqlxError::RowNotFound),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "database operation failed",
        ),
    ];

    for (error, status, message) in cases {
        assert_eq!(error.to_string(), message);
        let response = error.into_response();
        assert_eq!(response.status(), status);
    }
}
