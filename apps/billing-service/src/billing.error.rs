use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    #[error("authentication required")]
    Unauthorized,
    #[error("Identity activity verification is unavailable")]
    IdentityUnavailable,
    #[error("invalid DPoP proof")]
    InvalidProof,
    #[error("DPoP verification unavailable")]
    ProofUnavailable,
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
            Self::InvalidProof => (StatusCode::UNAUTHORIZED, "invalid_dpop_proof"),
            Self::ProofUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "dpop_unavailable"),
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
        let mut response = (status, Json(ErrorBody { code, message })).into_response();
        if status == StatusCode::UNAUTHORIZED {
            let challenge = if matches!(self, Self::InvalidProof) {
                "DPoP error=\"invalid_dpop_proof\", algs=\"ES256\""
            } else {
                "Bearer, DPoP algs=\"ES256\""
            };
            response
                .headers_mut()
                .insert("www-authenticate", challenge.parse().unwrap());
        }
        response
    }
}

pub type BillingResult<T> = Result<T, BillingError>;

impl From<nvbes_dpop::resource::ResourceError> for BillingError {
    fn from(error: nvbes_dpop::resource::ResourceError) -> Self {
        match error {
            nvbes_dpop::resource::ResourceError::Invalid => Self::InvalidProof,
            _ => Self::ProofUnavailable,
        }
    }
}
