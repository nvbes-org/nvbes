#[path = "client.rs"]
mod client;
#[path = "config.rs"]
mod config;
#[path = "event.rs"]
mod event;
#[path = "privacy.rs"]
mod privacy;

pub use client::ProductAnalytics;
pub use config::{ProductAnalyticsConfig, ProductAnalyticsError};
pub use event::{AnalyticsProperties, ProductAnalyticsEvent, properties, size_bytes_bucket};
