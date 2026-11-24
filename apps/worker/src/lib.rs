//! NVBES Unified Background Worker.

#[path = "worker.config.rs"]
pub mod config;
#[path = "worker.health.rs"]
pub mod health;
#[path = "worker.privacy.rs"]
pub mod privacy;
#[path = "worker.server.rs"]
pub mod server;

pub use config::WorkerConfig;
pub use server::run_worker;

#[cfg(test)]
#[path = "worker.tests.rs"]
mod tests;
