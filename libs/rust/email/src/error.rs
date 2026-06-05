use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("email send failed ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("configuration error: {0}")]
    Config(String),
}

impl EmailError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Api { status, .. } => *status == 429 || *status >= 500,
            Self::Http(_) => true,
            Self::Serialization(_) => false,
            Self::Config(_) => false,
        }
    }
}
