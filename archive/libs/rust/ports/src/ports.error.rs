use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PortError {
    #[error("port operation failed: {0}")]
    OperationFailed(String),
    #[error("resource was not found")]
    NotFound,
    #[error("port operation is not implemented by this adapter")]
    Unsupported,
}
