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

    #[error("Identity access token validation failed: {0}")]
    TokenValidation(String),

    #[error("Identity JWKS resolution failed: {0}")]
    Jwks(String),

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

    pub fn invalid_token(message: impl Into<String>) -> Self {
        Self::TokenValidation(message.into())
    }

    pub fn jwks(message: impl Into<String>) -> Self {
        Self::Jwks(message.into())
    }
}
