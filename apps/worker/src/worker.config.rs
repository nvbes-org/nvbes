use std::{net::SocketAddr, time::Duration};

use crate::tasks::WorkerTask;

pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required configuration: {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// Settings shared by `serve` and `run`. Outside `development`/`test` nothing
/// is defaulted that could point the worker at the wrong database.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerConfig {
    pub environment: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
    pub observability_internal_token: Option<String>,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(&env_lookup)
    }

    pub fn from_lookup(lookup: Lookup<'_>) -> Result<Self, ConfigError> {
        let optional = |name: &str| present(lookup, name);
        let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
        let development = is_development(&environment);
        let database_url = optional("DATABASE_URL")
            .or_else(|| optional("NVBES_WORKER_DATABASE_URL"))
            .or_else(|| development.then(|| "postgres://localhost/nvbes_monolith".into()))
            .ok_or(ConfigError::Missing("DATABASE_URL"))?;
        Ok(Self {
            database_max_connections: number(lookup, "NVBES_WORKER_DATABASE_MAX_CONNECTIONS", 2)?,
            database_min_connections: number(lookup, "NVBES_WORKER_DATABASE_MIN_CONNECTIONS", 0)?,
            sentry_dsn: optional("SENTRY_DSN").or_else(|| optional("NVBES_WORKER_SENTRY_DSN")),
            sentry_traces_sample_rate: optional("SENTRY_TRACES_SAMPLE_RATE")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.1),
            otlp_endpoint: optional("OTEL_EXPORTER_OTLP_ENDPOINT")
                .or_else(|| optional("NVBES_WORKER_OTLP_ENDPOINT")),
            otlp_authorization_header: optional("OTEL_EXPORTER_OTLP_HEADERS")
                .or_else(|| optional("NVBES_WORKER_OTLP_AUTHORIZATION")),
            observability_internal_token: optional("NVBES_OBSERVABILITY_INTERNAL_TOKEN")
                .or_else(|| optional("OBSERVABILITY_INTERNAL_TOKEN")),
            environment,
            database_url,
        })
    }

    pub fn development(&self) -> bool {
        is_development(&self.environment)
    }
}

/// A recurring task and how often `serve` runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    pub task: WorkerTask,
    pub every: Duration,
}

/// Settings only the long-running Container mode needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeConfig {
    pub bind_addr: SocketAddr,
    pub schedules: Vec<Schedule>,
}

impl ServeConfig {
    pub fn from_lookup(lookup: Lookup<'_>, development: bool) -> Result<Self, ConfigError> {
        let port: u16 = number(lookup, "PORT", 8081)?;
        let host = if development { "127.0.0.1" } else { "0.0.0.0" };
        let bind_addr = SocketAddr::new(
            host.parse()
                .map_err(|_| ConfigError::Invalid("PORT".into()))?,
            port,
        );
        const TASKS: &str = "NVBES_WORKER_TASKS";
        let tasks = match present(lookup, TASKS) {
            Some(tasks) => tasks,
            None if development => "identity-emails,privacy-retention".into(),
            None => return Err(ConfigError::Missing(TASKS)),
        };
        let mut schedules: Vec<Schedule> = Vec::new();
        for name in tasks
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let task: WorkerTask = name.parse().map_err(|error: crate::tasks::UnknownTask| {
                ConfigError::Invalid(error.to_string())
            })?;
            if !task.recurring() {
                return Err(ConfigError::Invalid(format!(
                    "{TASKS}: {task} is one-shot; run it with `nvbes-worker run {task}`"
                )));
            }
            if schedules.iter().any(|schedule| schedule.task == task) {
                return Err(ConfigError::Invalid(format!(
                    "{TASKS}: {task} is listed twice"
                )));
            }
            let seconds = match task.interval_variable() {
                Some(name) => number(lookup, name, task.default_interval_secs())?,
                None => task.default_interval_secs(),
            };
            if seconds == 0 {
                return Err(ConfigError::Invalid(format!(
                    "{task} interval must be positive"
                )));
            }
            schedules.push(Schedule {
                task,
                every: Duration::from_secs(seconds),
            });
        }
        if schedules.is_empty() {
            return Err(ConfigError::Invalid(format!("{TASKS} lists no task")));
        }
        Ok(Self {
            bind_addr,
            schedules,
        })
    }
}

pub fn is_development(environment: &str) -> bool {
    matches!(environment, "development" | "test")
}

/// Non-UTF-8 values stay present (lossily decoded) so they fail validation
/// instead of being mistaken for an absent variable.
pub fn env_lookup(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

pub fn present(lookup: Lookup<'_>, name: &str) -> Option<String> {
    lookup(name)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn number<T: std::str::FromStr>(
    lookup: Lookup<'_>,
    name: &'static str,
    default: T,
) -> Result<T, ConfigError> {
    match present(lookup, name) {
        Some(value) => value.parse().map_err(|_| ConfigError::Invalid(name.into())),
        None => Ok(default),
    }
}

#[cfg(test)]
#[path = "worker.config.tests.rs"]
mod tests;
