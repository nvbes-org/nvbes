#[cfg(feature = "http")]
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConsentError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Document not found or unsupported: {0}")]
    UnsupportedDocument(String),

    #[error("Audit log error: {0}")]
    Audit(String),
}

#[cfg(feature = "http")]
#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

#[cfg(feature = "http")]
impl IntoResponse for ConsentError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidInput(msg) => (StatusCode::BAD_REQUEST, "invalid_input", msg),
            Self::UnsupportedDocument(msg) => {
                (StatusCode::BAD_REQUEST, "unsupported_document", msg)
            }
            Self::Database(err) => {
                tracing::error!(error = %err, "Database error in consent service");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error.".to_string(),
                )
            }
            Self::Audit(err) => {
                tracing::error!(error = %err, "Audit error in consent service");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error.".to_string(),
                )
            }
        };

        (status, Json(ErrorBody { code, message })).into_response()
    }
}
