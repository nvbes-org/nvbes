#[path = "account.app.rs"]
pub mod app;
#[path = "account.audit.rs"]
pub mod audit;
#[path = "account.auth.rs"]
pub mod auth;
#[path = "account.billing_authorization.rs"]
pub mod billing_authorization;
#[path = "account.config.rs"]
pub mod config;
#[path = "account.consents.rs"]
pub mod consents;
#[path = "account.database.rs"]
pub mod database;
#[path = "account.error.rs"]
pub mod error;
#[path = "account.health.rs"]
pub mod health;
#[path = "account.metrics.rs"]
pub mod metrics;
#[path = "account.operator.rs"]
pub mod operator;
#[path = "account.outbox.rs"]
pub mod outbox;
#[path = "account.preferences.rs"]
pub mod preferences;
#[path = "account.privacy.rs"]
pub mod privacy;
#[path = "account.privacy.jobs.rs"]
pub mod privacy_jobs;
#[path = "account.profile.rs"]
pub mod profile;
#[path = "account.synthetic.rs"]
pub mod synthetic;
#[path = "account.teams.rs"]
pub mod teams;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "test harness helpers referenced across account test modules"
)]
#[path = "account.test_support.rs"]
pub mod test_support;

pub use app::{AccountState, router};
pub use config::AccountConfig;
pub use error::AccountError;
