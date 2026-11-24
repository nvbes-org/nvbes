use std::net::SocketAddr;

use ipnet::IpNet;
use nvbes_account_service::AccountConfig;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required configuration: {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(&'static str),
    #[error("invalid identity configuration: {0}")]
    Identity(String),
}

/// Everything the API needs to serve, loaded once and validated before the
/// listener binds. Outside `development`/`test` no secret is ever generated.
#[derive(Clone)]
pub struct ApiConfig {
    pub bind_addr: SocketAddr,
    pub environment: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
    pub observability_internal_token: Option<String>,
    /// Reverse proxies whose `X-Forwarded-For` hop is trusted for client IPs.
    pub trusted_proxy_cidrs: Vec<String>,
    pub account: AccountConfig,
}

impl ApiConfig {
    pub fn development(&self) -> bool {
        is_development(&self.environment)
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(&env_lookup)
    }

    /// `lookup` returns the raw value of a variable; blank values count as absent.
    pub fn from_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let optional = |name: &str| present(lookup, name);
        let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
        let development = is_development(&environment);

        let port_str = optional("PORT").or_else(|| optional("NVBES_API_PORT"));
        let port: u16 = match port_str {
            Some(p) => p.parse().map_err(|_| ConfigError::Invalid("PORT"))?,
            None => 8080,
        };
        let host = if development { "127.0.0.1" } else { "0.0.0.0" };
        let bind_addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|_| ConfigError::Invalid("PORT"))?;

        let database_url = optional("DATABASE_URL")
            .or_else(|| optional("NVBES_API_DATABASE_URL"))
            .or_else(|| development.then(|| "postgres://localhost/nvbes_monolith".into()))
            .ok_or(ConfigError::Missing("DATABASE_URL"))?;

        let database_max_connections = optional("NVBES_API_DATABASE_MAX_CONNECTIONS")
            .map(|val| {
                val.parse::<u32>()
                    .map_err(|_| ConfigError::Invalid("NVBES_API_DATABASE_MAX_CONNECTIONS"))
            })
            .transpose()?
            .unwrap_or(10);

        let database_min_connections = optional("NVBES_API_DATABASE_MIN_CONNECTIONS")
            .map(|val| {
                val.parse::<u32>()
                    .map_err(|_| ConfigError::Invalid("NVBES_API_DATABASE_MIN_CONNECTIONS"))
            })
            .transpose()?
            .unwrap_or(0);

        let sentry_dsn = optional("SENTRY_DSN").or_else(|| optional("NVBES_API_SENTRY_DSN"));
        let sentry_traces_sample_rate = optional("SENTRY_TRACES_SAMPLE_RATE")
            .and_then(|val| val.parse::<f32>().ok())
            .unwrap_or(0.1);

        let otlp_endpoint =
            optional("OTEL_EXPORTER_OTLP_ENDPOINT").or_else(|| optional("NVBES_API_OTLP_ENDPOINT"));
        let otlp_authorization_header = optional("OTEL_EXPORTER_OTLP_HEADERS")
            .or_else(|| optional("NVBES_API_OTLP_AUTHORIZATION"));
        let observability_internal_token = optional("NVBES_OBSERVABILITY_INTERNAL_TOKEN")
            .or_else(|| optional("OBSERVABILITY_INTERNAL_TOKEN"));

        let trusted_proxy_cidrs = trusted_proxy_cidrs(lookup, development)?;

        let account =
            AccountConfig::from_lookup(development, lookup).map_err(|error| match error {
                nvbes_account_service::config::ConfigError::Missing(name) => {
                    ConfigError::Missing(name)
                }
                nvbes_account_service::config::ConfigError::Invalid(name) => {
                    ConfigError::Invalid(name)
                }
            })?;

        Ok(Self {
            bind_addr,
            environment,
            database_url,
            database_max_connections,
            database_min_connections,
            sentry_dsn,
            sentry_traces_sample_rate,
            otlp_endpoint,
            otlp_authorization_header,
            observability_internal_token,
            trusted_proxy_cidrs,
            account,
        })
    }
}

pub(crate) fn is_development(environment: &str) -> bool {
    matches!(environment, "development" | "test")
}

/// Outside development the deployment must name its proxies explicitly; an
/// absent list would make every caller share the proxy's address and thus a
/// single per-source rate-limit bucket.
fn trusted_proxy_cidrs(
    lookup: &dyn Fn(&str) -> Option<String>,
    development: bool,
) -> Result<Vec<String>, ConfigError> {
    const NAME: &str = "NVBES_TRUSTED_PROXY_CIDRS";
    let Some(raw) = present(lookup, NAME) else {
        return if development {
            Ok(Vec::new())
        } else {
            Err(ConfigError::Missing(NAME))
        };
    };
    raw.split(',')
        .map(str::trim)
        .filter(|cidr| !cidr.is_empty())
        .map(|cidr| {
            cidr.parse::<IpNet>()
                .map(|net| net.to_string())
                .map_err(|_| ConfigError::Invalid(NAME))
        })
        .collect()
}

/// Non-UTF-8 values stay present (lossily decoded) so they fail validation
/// instead of being mistaken for an absent variable.
pub fn env_lookup(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

pub(crate) fn present(lookup: &dyn Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    lookup(name)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "api.config.tests.rs"]
mod tests;
