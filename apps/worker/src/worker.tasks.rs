//! Units of background work. Each task is one bounded batch so it runs the
//! same way in a Scaleway Job (`run <task>`, then exit) and in the Scaleway
//! Container loop (`serve`).

use std::{fmt, str::FromStr};

use nvbes_email::DirectEmailDelivery;
use nvbes_identity_service::{mfa_crypto::MfaCrypto, mfa_keys::MfaKeys};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::config::{ConfigError, Lookup};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkerTask {
    /// Applies every domain migration; one-shot, run before each deploy.
    Migrate,
    /// Delivers queued security notifications and password-recovery links.
    IdentityEmails,
    /// Completes due RGPD exports and account closures.
    PrivacyRetention,
    /// Re-seals MFA secrets under the active key; one-shot, after a key change.
    RotateMfaKey,
}

impl WorkerTask {
    pub const ALL: [WorkerTask; 4] = [
        Self::Migrate,
        Self::IdentityEmails,
        Self::PrivacyRetention,
        Self::RotateMfaKey,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Migrate => "migrate",
            Self::IdentityEmails => "identity-emails",
            Self::PrivacyRetention => "privacy-retention",
            Self::RotateMfaKey => "rotate-mfa-key",
        }
    }

    /// Only recurring batches may be scheduled by `serve`.
    pub fn recurring(self) -> bool {
        matches!(self, Self::IdentityEmails | Self::PrivacyRetention)
    }

    pub fn interval_variable(self) -> Option<&'static str> {
        match self {
            Self::IdentityEmails => Some("NVBES_WORKER_IDENTITY_EMAILS_INTERVAL_SECS"),
            Self::PrivacyRetention => Some("NVBES_WORKER_PRIVACY_RETENTION_INTERVAL_SECS"),
            Self::Migrate | Self::RotateMfaKey => None,
        }
    }

    pub fn default_interval_secs(self) -> u64 {
        match self {
            Self::IdentityEmails => 15,
            Self::PrivacyRetention => 300,
            Self::Migrate | Self::RotateMfaKey => 0,
        }
    }
}

impl fmt::Display for WorkerTask {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown worker task: {0}")]
pub struct UnknownTask(pub String);

impl FromStr for WorkerTask {
    type Err = UnknownTask;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|task| task.name() == value)
            .ok_or_else(|| UnknownTask(value.to_owned()))
    }
}

/// A task with every dependency it needs, built once at start-up so a
/// misconfiguration stops the process before any batch runs.
pub enum PreparedTask {
    Migrate,
    IdentityEmails {
        crypto: MfaCrypto,
        delivery: DirectEmailDelivery,
    },
    PrivacyRetention,
    RotateMfaKey {
        crypto: MfaCrypto,
    },
}

impl PreparedTask {
    pub fn prepare(
        task: WorkerTask,
        lookup: Lookup<'_>,
        development: bool,
    ) -> Result<Self, ConfigError> {
        Ok(match task {
            WorkerTask::Migrate => Self::Migrate,
            WorkerTask::IdentityEmails => Self::IdentityEmails {
                crypto: mfa_crypto(lookup)?,
                delivery: crate::mailer::from_lookup(lookup, development)?,
            },
            WorkerTask::PrivacyRetention => Self::PrivacyRetention,
            WorkerTask::RotateMfaKey => Self::RotateMfaKey {
                crypto: mfa_crypto(lookup)?,
            },
        })
    }

    pub fn task(&self) -> WorkerTask {
        match self {
            Self::Migrate => WorkerTask::Migrate,
            Self::IdentityEmails { .. } => WorkerTask::IdentityEmails,
            Self::PrivacyRetention => WorkerTask::PrivacyRetention,
            Self::RotateMfaKey { .. } => WorkerTask::RotateMfaKey,
        }
    }

    /// Runs one batch and returns a log-safe summary. The future is boxed as
    /// `Send` so the serve loop can spawn it on the runtime.
    pub fn execute<'a>(&'a self, pool: &'a PgPool) -> TaskFuture<'a> {
        Box::pin(self.run(pool))
    }

    async fn run(&self, pool: &PgPool) -> anyhow::Result<Value> {
        match self {
            Self::Migrate => {
                crate::migrations::run_migrations(pool).await?;
                Ok(json!({"migrated": true}))
            }
            Self::IdentityEmails { crypto, delivery } => {
                let notifications =
                    nvbes_identity_service::notification_dispatch::run_batch(pool, delivery)
                        .await?;
                let recoveries =
                    nvbes_identity_service::recovery_delivery::run_batch(pool, crypto, delivery)
                        .await?;
                Ok(json!({
                    "security_notifications": notifications,
                    "password_recoveries": recoveries,
                }))
            }
            Self::PrivacyRetention => {
                let result = nvbes_account_service::privacy_jobs::process_pending(pool).await?;
                Ok(json!({
                    "exports_completed": result.exports_completed,
                    "closures_completed": result.closures_completed,
                }))
            }
            Self::RotateMfaKey { crypto } => {
                let rotated = nvbes_identity_service::mfa_rotation::rotate(pool, crypto).await?;
                Ok(json!({"rotated_factors": rotated}))
            }
        }
    }
}

/// The worker must read exactly the keys the API sealed with, so it never
/// falls back to a generated key, not even in development.
fn mfa_crypto(lookup: Lookup<'_>) -> Result<MfaCrypto, ConfigError> {
    let keys = MfaKeys::from_lookup(|name| crate::config::present(lookup, name))
        .map_err(|error| ConfigError::Invalid(error.to_string()))?;
    keys.crypto()
        .map_err(|error| ConfigError::Invalid(error.to_string()))
}

pub type TaskFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Value>> + Send + 'a>>;

#[cfg(test)]
#[path = "worker.tasks.tests.rs"]
mod tests;
