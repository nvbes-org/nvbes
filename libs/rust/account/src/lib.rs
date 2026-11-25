#[path = "account.app.rs"]
pub mod app;
#[path = "account.audit.rs"]
pub mod audit;
#[path = "account.auth.rs"]
pub mod auth;
#[path = "account.config.rs"]
pub mod config;
#[path = "account.consents.rs"]
pub mod consents;
#[path = "account.database.rs"]
pub mod database;
#[path = "account.error.rs"]
pub mod error;
#[path = "account.operator.rs"]
pub mod operator;
#[path = "account.preferences.rs"]
pub mod preferences;
#[path = "account.privacy.rs"]
pub mod privacy;
#[path = "account.privacy.jobs.rs"]
pub mod privacy_jobs;
#[path = "account.profile.rs"]
pub mod profile;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "test harness helpers referenced across account test modules"
)]
#[path = "account.test_support.rs"]
pub mod test_support;

pub use app::{AccountState, domain_router};
pub use auth::{ACCOUNT_AUDIENCE, TokenTrust};
pub use config::AccountConfig;
pub use error::AccountError;
