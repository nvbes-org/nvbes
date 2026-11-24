//! nvbes background worker: one binary for Scaleway Containers (`serve`) and
//! Scaleway Jobs (`run <task>`).

#[path = "worker.cli.rs"]
pub mod cli;
#[path = "worker.config.rs"]
pub mod config;
#[path = "worker.health.rs"]
pub mod health;
#[path = "worker.mailer.rs"]
pub mod mailer;
#[path = "worker.migrations.rs"]
pub mod migrations;
#[path = "worker.server.rs"]
pub mod server;
#[path = "worker.tasks.rs"]
pub mod tasks;

pub use cli::Command;
pub use config::WorkerConfig;
pub use tasks::WorkerTask;

#[cfg(test)]
#[path = "worker.tests.rs"]
mod tests;
