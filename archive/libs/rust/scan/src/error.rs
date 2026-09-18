use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("ClamAV connection failed: {0}")]
    ConnectionFailed(String),

    #[error("ClamAV scan timeout after {0} seconds")]
    Timeout(u64),

    #[error("ClamAV protocol error: {0}")]
    ProtocolError(String),

    #[error("File too large for scanning: {0} bytes (max: {1} bytes)")]
    FileTooLarge(i64, i64),

    #[error("Scanner IO error: {0}")]
    Io(#[from] std::io::Error),
}
