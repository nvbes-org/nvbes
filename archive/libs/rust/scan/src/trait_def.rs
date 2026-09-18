use async_trait::async_trait;

use crate::error::ScanError;

#[derive(Debug, Clone)]
pub enum ScanResult {
    Clean,
    Infected { signature: String },
}

#[async_trait]
pub trait ScanEngine: Send + Sync {
    async fn scan(&self, data: &[u8]) -> Result<ScanResult, ScanError>;
}
