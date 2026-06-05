use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Authentication failed: {message}")]
    Auth {
        message: String,
        status: Option<u16>,
    },

    #[error("Token exchange failed: {0}")]
    TokenExchange(String),

    #[error("MFA required: {0}")]
    MfaRequired(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl SdkError {
    pub fn auth(message: impl Into<String>, status: Option<u16>) -> Self {
        Self::Auth {
            message: message.into(),
            status,
        }
    }
}
