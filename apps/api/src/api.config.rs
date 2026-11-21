use std::net::SocketAddr;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required configuration: {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
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
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
        let development = matches!(environment.as_str(), "development" | "test");

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
        })
    }
}

fn optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}
