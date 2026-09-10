use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("authentication required")]
    Unauthorized,
    #[error("Identity activity verification is unavailable")]
    IdentityUnavailable,
    #[error("insufficient scope")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("request conflicts with current state")]
    Conflict,
    #[error("invalid request: {0}")]
    Invalid(&'static str),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AccountError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "authentication_required"),
            Self::IdentityUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "identity_unavailable"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "insufficient_scope"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict => (StatusCode::CONFLICT, "state_conflict"),
            Self::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        let message = self.to_string();
        (status, Json(ErrorBody { code, message })).into_response()
    }
}

pub type AccountResult<T> = Result<T, AccountError>;
