mod clamav;
mod error;
mod mock;
mod trait_def;

pub use clamav::ClamAvScanner;
pub use error::ScanError;
pub use mock::MockScanner;
pub use trait_def::{ScanEngine, ScanResult};
