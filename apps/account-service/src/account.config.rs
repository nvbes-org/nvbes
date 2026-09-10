use std::net::SocketAddr;

#[derive(Clone, PartialEq)]
pub struct AccountConfig {
    pub billing_authorization_secret: Option<String>,
    pub environment: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub bind_addr: SocketAddr,
    pub token_issuer: String,
    pub token_audience: String,
    pub token_key_id: String,
    pub token_public_key_pem: String,
    pub identity_resource_client_id: String,
    pub identity_resource_secret: String,
    pub metrics_token: String,
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
}

impl AccountConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let billing_authorization_secret = optional("NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET");
        if billing_authorization_secret
            .as_deref()
            .is_some_and(|s| !crate::billing_authorization::valid_secret(s))
        {
            return Err(ConfigError::Invalid(
                "NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET",
            ));
        }
        let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
        let development = matches!(environment.as_str(), "development" | "test");
        let database_url = optional("NVBES_ACCOUNT_DATABASE_URL")
            .or_else(|| development.then(|| "postgres://localhost/nvbes_account".into()))
            .ok_or(ConfigError::Missing("NVBES_ACCOUNT_DATABASE_URL"))?;
        let database_max_connections = parsed_range(
            "NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS",
            development.then_some("5"),
            1..=20,
        )?;
        let default_bind = std::env::var("PORT")
            .map(|port| format!("0.0.0.0:{port}"))
            .unwrap_or_else(|_| "127.0.0.1:3070".into());
        let bind_addr = optional("NVBES_ACCOUNT_BIND_ADDR")
            .unwrap_or(default_bind)
            .parse()
            .map_err(|_| ConfigError::Invalid("NVBES_ACCOUNT_BIND_ADDR"))?;
        let token_issuer = optional("NVBES_IDENTITY_TOKEN_ISSUER")
            .or_else(|| development.then(|| "http://identity.local".into()))
            .ok_or(ConfigError::Missing("NVBES_IDENTITY_TOKEN_ISSUER"))?;
        let token_audience = optional("NVBES_ACCOUNT_TOKEN_AUDIENCE")
            .or_else(|| development.then(|| "nvbes-account-service".into()))
            .ok_or(ConfigError::Missing("NVBES_ACCOUNT_TOKEN_AUDIENCE"))?;
        let token_key_id = required("NVBES_IDENTITY_TOKEN_KEY_ID")?;
        let token_public_key_pem = required("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM")?;
        let identity_resource_client_id = required("NVBES_ACCOUNT_IDENTITY_RESOURCE_CLIENT_ID")?;
        let identity_resource_secret = required("NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET")?;
        validate_token_contract(development, &token_issuer, &token_audience, &token_key_id)?;
        let metrics_token = optional("NVBES_ACCOUNT_METRICS_TOKEN")
            .or_else(|| development.then(|| "development-account-metrics-token-value".into()))
            .ok_or(ConfigError::Missing("NVBES_ACCOUNT_METRICS_TOKEN"))?;
        let sentry_dsn = optional("SENTRY_DSN");
        let sentry_traces_sample_rate = optional("SENTRY_TRACES_SAMPLE_RATE")
            .unwrap_or_else(|| "0.1".into())
            .parse::<f32>()
            .ok()
            .filter(|value| (0.0..=1.0).contains(value))
            .ok_or(ConfigError::Invalid("SENTRY_TRACES_SAMPLE_RATE"))?;
        let otlp_endpoint = optional("NVBES_OTLP_ENDPOINT");
        let otlp_authorization_header = optional("NVBES_OTLP_AUTHORIZATION_HEADER");
        validate_observability(
            development,
            sentry_dsn.as_deref(),
            otlp_endpoint.as_deref(),
            otlp_authorization_header.as_deref(),
            &metrics_token,
        )?;
        Ok(Self {
            billing_authorization_secret,
            environment,
            database_url,
            database_max_connections,
            bind_addr,
            token_issuer,
            token_audience,
            token_key_id,
            token_public_key_pem,
            identity_resource_client_id,
            identity_resource_secret,
            metrics_token,
            sentry_dsn,
            sentry_traces_sample_rate,
            otlp_endpoint,
            otlp_authorization_header,
        })
    }
}

fn validate_token_contract(
    development: bool,
    issuer: &str,
    audience: &str,
    key_id: &str,
) -> Result<(), ConfigError> {
    if audience != "nvbes-account-service" {
        return Err(ConfigError::Invalid("NVBES_ACCOUNT_TOKEN_AUDIENCE"));
    }
    let parsed = issuer
        .parse::<axum::http::Uri>()
        .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_TOKEN_ISSUER"))?;
    if parsed.host().is_none() || (!development && parsed.scheme_str() != Some("https")) {
        return Err(ConfigError::Invalid("NVBES_IDENTITY_TOKEN_ISSUER"));
    }
    for (name, value) in [
        ("NVBES_ACCOUNT_TOKEN_AUDIENCE", audience),
        ("NVBES_IDENTITY_TOKEN_KEY_ID", key_id),
    ] {
        if !(3..=128).contains(&value.len())
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
        {
            return Err(ConfigError::Invalid(name));
        }
    }
    Ok(())
}

fn validate_observability(
    development: bool,
    sentry: Option<&str>,
    otlp: Option<&str>,
    authorization: Option<&str>,
    metrics_token: &str,
) -> Result<(), ConfigError> {
    if metrics_token.len() < 32 || metrics_token.contains(['\r', '\n']) {
        return Err(ConfigError::Invalid("NVBES_ACCOUNT_METRICS_TOKEN"));
    }
    if !development {
        if !sentry.is_some_and(|value| value.starts_with("https://") && value.contains('@')) {
            return Err(ConfigError::Invalid("SENTRY_DSN"));
        }
        if !otlp.is_some_and(|value| value.starts_with("https://")) {
            return Err(ConfigError::Invalid("NVBES_OTLP_ENDPOINT"));
        }
        if !authorization
            .is_some_and(|value| value.starts_with("Basic ") && !value.contains(['\r', '\n']))
        {
            return Err(ConfigError::Invalid("NVBES_OTLP_AUTHORIZATION_HEADER"));
        }
    }
    Ok(())
}

fn parsed_range(
    name: &'static str,
    default: Option<&str>,
    range: std::ops::RangeInclusive<u32>,
) -> Result<u32, ConfigError> {
    optional(name)
        .or_else(|| default.map(str::to_owned))
        .and_then(|value| value.parse().ok())
        .filter(|value| range.contains(value))
        .ok_or(ConfigError::Invalid(name))
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    optional(name).ok_or(ConfigError::Missing(name))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("required configuration is missing: {0}")]
    Missing(&'static str),
    #[error("configuration is invalid: {0}")]
    Invalid(&'static str),
}
