use std::net::SocketAddr;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required configuration: {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkerConfig {
    pub bind_addr: SocketAddr,
    pub environment: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub poll_interval_secs: u64,
    pub privacy_interval_secs: u64,
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
    pub observability_internal_token: Option<String>,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
        let development = matches!(environment.as_str(), "development" | "test");

        let port_str = optional("PORT").or_else(|| optional("NVBES_WORKER_PORT"));
        let port: u16 = match port_str {
            Some(p) => p.parse().map_err(|_| ConfigError::Invalid("PORT"))?,
            None => 8081,
        };
        let host = if development { "127.0.0.1" } else { "0.0.0.0" };
        let bind_addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|_| ConfigError::Invalid("PORT"))?;

        let database_url = optional("DATABASE_URL")
            .or_else(|| optional("NVBES_WORKER_DATABASE_URL"))
            .or_else(|| development.then(|| "postgres://localhost/nvbes_monolith".into()))
            .ok_or(ConfigError::Missing("DATABASE_URL"))?;

        let database_max_connections = optional("NVBES_WORKER_DATABASE_MAX_CONNECTIONS")
            .map(|val| {
                val.parse::<u32>()
                    .map_err(|_| ConfigError::Invalid("NVBES_WORKER_DATABASE_MAX_CONNECTIONS"))
            })
            .transpose()?
            .unwrap_or(5);

        let database_min_connections = optional("NVBES_WORKER_DATABASE_MIN_CONNECTIONS")
            .map(|val| {
                val.parse::<u32>()
                    .map_err(|_| ConfigError::Invalid("NVBES_WORKER_DATABASE_MIN_CONNECTIONS"))
            })
            .transpose()?
            .unwrap_or(0);

        let poll_interval_secs = optional("NVBES_WORKER_POLL_INTERVAL_SECS")
            .map(|val| {
                val.parse::<u64>()
                    .map_err(|_| ConfigError::Invalid("NVBES_WORKER_POLL_INTERVAL_SECS"))
            })
            .transpose()?
            .unwrap_or(5);

        let privacy_interval_secs = optional("NVBES_WORKER_PRIVACY_INTERVAL_SECS")
            .map(|val| {
                val.parse::<u64>()
                    .map_err(|_| ConfigError::Invalid("NVBES_WORKER_PRIVACY_INTERVAL_SECS"))
            })
            .transpose()?
            .unwrap_or(60);

        let sentry_dsn = optional("SENTRY_DSN").or_else(|| optional("NVBES_WORKER_SENTRY_DSN"));
        let sentry_traces_sample_rate = optional("SENTRY_TRACES_SAMPLE_RATE")
            .and_then(|val| val.parse::<f32>().ok())
            .unwrap_or(0.1);

        let otlp_endpoint = optional("OTEL_EXPORTER_OTLP_ENDPOINT")
            .or_else(|| optional("NVBES_WORKER_OTLP_ENDPOINT"));
        let otlp_authorization_header = optional("OTEL_EXPORTER_OTLP_HEADERS")
            .or_else(|| optional("NVBES_WORKER_OTLP_AUTHORIZATION"));
        let observability_internal_token = optional("NVBES_OBSERVABILITY_INTERNAL_TOKEN")
            .or_else(|| optional("OBSERVABILITY_INTERNAL_TOKEN"));

        Ok(Self {
            bind_addr,
            environment,
            database_url,
            database_max_connections,
            database_min_connections,
            poll_interval_secs,
            privacy_interval_secs,
            sentry_dsn,
            sentry_traces_sample_rate,
            otlp_endpoint,
            otlp_authorization_header,
            observability_internal_token,
        })
    }
}

fn optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}
