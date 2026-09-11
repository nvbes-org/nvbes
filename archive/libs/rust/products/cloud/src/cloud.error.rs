#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    #[error("configuration_error: {0}")]
    Configuration(String),
    #[error("database_error: {0}")]
    Database(#[from] sqlx::Error),
}

pub type CloudResult<T> = Result<T, CloudError>;
