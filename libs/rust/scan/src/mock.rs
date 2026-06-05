use async_trait::async_trait;
use tracing::debug;

use crate::error::ScanError;
use crate::trait_def::{ScanEngine, ScanResult};

pub struct MockScanner {
    force_infected: bool,
}

impl MockScanner {
    pub fn new() -> Self {
        Self {
            force_infected: false,
        }
    }

    pub fn force_infected(mut self) -> Self {
        self.force_infected = true;
        self
    }
}

impl Default for MockScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ScanEngine for MockScanner {
    async fn scan(&self, data: &[u8]) -> Result<ScanResult, ScanError> {
        debug!(size = data.len(), "MockScanner: scan (no-op)");

        if self.force_infected {
            Ok(ScanResult::Infected {
                signature: "Mock.TestVirus".to_string(),
            })
        } else {
            Ok(ScanResult::Clean)
        }
    }
}
