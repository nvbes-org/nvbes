#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("invalid token configuration: {0}")]
    Configuration(&'static str),
    #[error("invalid token")]
    InvalidToken,
    #[error("grant is inactive or already issued")]
    InactiveGrant,
    #[error("invalid authentication evidence")]
    InvalidAuthentication,
    #[error("invalid token scope or audience")]
    InvalidPolicy,
    #[error("token cryptography failed")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("token storage unavailable")]
    Database(#[from] sqlx::Error),
    #[error("invalid stored authorization")]
    Authorization(#[from] crate::oauth::error::OAuthError),
}
