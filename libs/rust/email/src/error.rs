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

    #[error("email address error: {0}")]
    Address(#[from] lettre::address::AddressError),

    #[error("email build error: {0}")]
    Build(#[from] lettre::error::Error),

    #[error("smtp error: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),
}

impl EmailError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Api { status, .. } => *status == 429 || *status >= 500,
            Self::Http(_) => true,
            Self::Smtp(_) => true,
            Self::Serialization(_) => false,
            Self::Config(_) => false,
            Self::Address(_) => false,
            Self::Build(_) => false,
        }
    }
}
