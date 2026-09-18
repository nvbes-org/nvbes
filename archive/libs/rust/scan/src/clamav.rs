use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, warn};

use crate::error::ScanError;
use crate::trait_def::{ScanEngine, ScanResult};

const CHUNK_SIZE: usize = 64 * 1024;
const MAX_FILE_SIZE: i64 = 25 * 1024 * 1024;

pub struct ClamAvScanner {
    host: String,
    port: u16,
    timeout_secs: u64,
}

impl ClamAvScanner {
    pub fn new(host: String, port: u16, timeout_secs: u64) -> Self {
        Self {
            host,
            port,
            timeout_secs,
        }
    }

    async fn send_instream(&self, data: &[u8]) -> Result<String, ScanError> {
        let addr = format!("{}:{}", self.host, self.port);
        let duration = Duration::from_secs(self.timeout_secs);

        let mut stream = timeout(duration, TcpStream::connect(&addr))
            .await
            .map_err(|_| ScanError::Timeout(self.timeout_secs))?
            .map_err(|e| ScanError::ConnectionFailed(format!("{e}")))?;

        stream
            .write_all(b"zINSTREAM\0")
            .await
            .map_err(|e| ScanError::ConnectionFailed(format!("{e}")))?;

        let mut offset = 0;
        while offset < data.len() {
            let end = (offset + CHUNK_SIZE).min(data.len());
            let chunk_size = (end - offset) as u32;

            stream
                .write_all(&chunk_size.to_be_bytes())
                .await
                .map_err(|e| ScanError::ProtocolError(format!("write chunk size: {e}")))?;

            stream
                .write_all(&data[offset..end])
                .await
                .map_err(|e| ScanError::ProtocolError(format!("write chunk data: {e}")))?;

            offset = end;
        }

        stream
            .write_all(&0u32.to_be_bytes())
            .await
            .map_err(|e| ScanError::ProtocolError(format!("write terminator: {e}")))?;

        let mut response = Vec::new();
        timeout(duration, stream.read_to_end(&mut response))
            .await
            .map_err(|_| ScanError::Timeout(self.timeout_secs))?
            .map_err(|e| ScanError::ProtocolError(format!("read response: {e}")))?;

        String::from_utf8(response)
            .map(|s| s.trim().to_string())
            .map_err(|e| ScanError::ProtocolError(format!("invalid utf8 response: {e}")))
    }
}

#[async_trait]
impl ScanEngine for ClamAvScanner {
    async fn scan(&self, data: &[u8]) -> Result<ScanResult, ScanError> {
        if data.len() as i64 > MAX_FILE_SIZE {
            warn!(
                size = data.len(),
                max = MAX_FILE_SIZE,
                "File too large for ClamAV scanning, returning clean"
            );
            return Err(ScanError::FileTooLarge(data.len() as i64, MAX_FILE_SIZE));
        }

        debug!(size = data.len(), "Sending file to ClamAV for scanning");

        let response = self.send_instream(data).await?;

        debug!(response = %response, "ClamAV scan response received");

        if response.ends_with("OK") {
            Ok(ScanResult::Clean)
        } else if response.ends_with("FOUND") {
            let signature = response
                .split_whitespace()
                .next()
                .unwrap_or("unknown")
                .to_string();
            error!(signature = %signature, "Malware detected by ClamAV");
            Ok(ScanResult::Infected { signature })
        } else {
            Err(ScanError::ProtocolError(format!(
                "unexpected ClamAV response: {response}"
            )))
        }
    }
}
