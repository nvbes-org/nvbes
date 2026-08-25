const DEFAULT_SAMPLE_RATE: &str = "0";

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
}

pub fn from_environment(environment: &str) -> Result<ObservabilityConfig, super::ConfigError> {
    let config = ObservabilityConfig {
        sentry_dsn: optional_any(&["SENTRY_DSN", "NVBES_SENTRY_DSN"]),
        sentry_traces_sample_rate: parse_sample_rate(
            optional_any(&[
                "SENTRY_TRACES_SAMPLE_RATE",
                "NVBES_SENTRY_TRACES_SAMPLE_RATE",
            ])
            .as_deref(),
        )?,
        otlp_endpoint: optional("NVBES_OTLP_ENDPOINT"),
        otlp_authorization_header: optional("NVBES_OTLP_AUTHORIZATION_HEADER"),
    };
    validate(environment, &config)?;
    Ok(config)
}

pub(super) fn validate(
    environment: &str,
    config: &ObservabilityConfig,
) -> Result<(), super::ConfigError> {
    if !(0.0..=1.0).contains(&config.sentry_traces_sample_rate) {
        return Err(super::ConfigError::Invalid("SENTRY_TRACES_SAMPLE_RATE"));
    }
    if matches!(environment, "development" | "test") {
        return Ok(());
    }

    let sentry_dsn = config
        .sentry_dsn
        .as_deref()
        .ok_or(super::ConfigError::Missing("SENTRY_DSN"))?;
    if !safe_https_secret_url(sentry_dsn) || !sentry_dsn.contains('@') {
        return Err(super::ConfigError::Invalid("SENTRY_DSN"));
    }

    let otlp_endpoint = config
        .otlp_endpoint
        .as_deref()
        .ok_or(super::ConfigError::Missing("NVBES_OTLP_ENDPOINT"))?;
    if !safe_https_url(otlp_endpoint) {
        return Err(super::ConfigError::Invalid("NVBES_OTLP_ENDPOINT"));
    }

    let authorization =
        config
            .otlp_authorization_header
            .as_deref()
            .ok_or(super::ConfigError::Missing(
                "NVBES_OTLP_AUTHORIZATION_HEADER",
            ))?;
    if !authorization.starts_with("Basic ")
        || authorization.len() < 16
        || authorization.contains(['\r', '\n'])
    {
        return Err(super::ConfigError::Invalid(
            "NVBES_OTLP_AUTHORIZATION_HEADER",
        ));
    }
    Ok(())
}

pub(super) fn parse_sample_rate(value: Option<&str>) -> Result<f32, super::ConfigError> {
    let rate = value
        .unwrap_or(DEFAULT_SAMPLE_RATE)
        .parse::<f32>()
        .map_err(|_| super::ConfigError::Invalid("SENTRY_TRACES_SAMPLE_RATE"))?;
    if !(0.0..=1.0).contains(&rate) {
        return Err(super::ConfigError::Invalid("SENTRY_TRACES_SAMPLE_RATE"));
    }
    Ok(rate)
}

fn safe_https_url(value: &str) -> bool {
    value.starts_with("https://") && !value.contains(['\r', '\n'])
}

fn safe_https_secret_url(value: &str) -> bool {
    value == value.trim() && safe_https_url(value)
}

fn optional(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn optional_any(variables: &[&str]) -> Option<String> {
    variables.iter().find_map(|variable| optional(variable))
}
