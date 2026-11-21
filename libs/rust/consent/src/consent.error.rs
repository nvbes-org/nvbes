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
