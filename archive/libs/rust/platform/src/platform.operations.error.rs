use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum OperationsError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("case or cost not found")]
    NotFound,
    #[error("idempotency conflict or stale case version")]
    Conflict,
    #[error("permission denied")]
    Forbidden,
    #[error("storage unavailable")]
    Database(#[from] sqlx::Error),
    #[error("invalid persisted document")]
    Serialization(#[from] serde_json::Error),
}
impl IntoResponse for OperationsError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Database(_) | Self::Serialization(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status, Json(json!({"error": self.to_string()}))).into_response()
    }
}
