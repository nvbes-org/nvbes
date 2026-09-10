use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    #[error("authentication required")]
    Unauthorized,
    #[error("Identity activity verification is unavailable")]
    IdentityUnavailable,
    #[error("Account authorization is unavailable")]
    AccountUnavailable,
    #[error("billing account access denied")]
    AccountForbidden,
    #[error("insufficient scope")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("invalid request: {0}")]
    Invalid(&'static str),
    #[error("live mode rejected: {0}")]
    LiveModeRejected(&'static str),
    #[error("stripe provider error: {0}")]
    Stripe(String),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for BillingError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "authentication_required"),
            Self::IdentityUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "identity_unavailable"),
            Self::AccountUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "account_unavailable"),
            Self::AccountForbidden => (StatusCode::FORBIDDEN, "account_access_denied"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "insufficient_scope"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::LiveModeRejected(_) => (StatusCode::BAD_REQUEST, "live_mode_rejected"),
            Self::Stripe(_) => (StatusCode::BAD_GATEWAY, "stripe_error"),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        let message = self.to_string();
        (status, Json(ErrorBody { code, message })).into_response()
    }
}

pub type BillingResult<T> = Result<T, BillingError>;
