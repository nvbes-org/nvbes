const MINIMUM_INTERNAL_TOKEN_LENGTH: usize = 32;

pub struct ObservabilityConfig {
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
    pub observability_internal_token: Option<String>,
}

pub fn from_environment(environment: &str) -> anyhow::Result<ObservabilityConfig> {
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
        observability_internal_token: optional("NVBES_OBSERVABILITY_INTERNAL_TOKEN"),
    };
    validate(environment, &config)?;
    Ok(config)
}

fn validate(environment: &str, config: &ObservabilityConfig) -> anyhow::Result<()> {
    match config.sentry_dsn.as_deref() {
        None if environment == "production" => {
            anyhow::bail!("SENTRY_DSN is required in production")
        }
        Some(dsn) if dsn != dsn.trim() || !dsn.starts_with("https://") || !dsn.contains('@') => {
            anyhow::bail!("SENTRY_DSN must be a valid HTTPS Sentry DSN")
        }
        _ => {}
    }

    if environment != "development" && config.otlp_authorization_header.is_some() {
        anyhow::bail!(
            "NVBES_OTLP_AUTHORIZATION_HEADER must stay empty outside development; export OTLP through local Grafana Alloy"
        );
    }

    match config.observability_internal_token.as_deref() {
        None if environment != "development" => {
            anyhow::bail!("NVBES_OBSERVABILITY_INTERNAL_TOKEN is required outside development")
        }
        Some(token) if token.len() < MINIMUM_INTERNAL_TOKEN_LENGTH => anyhow::bail!(
            "NVBES_OBSERVABILITY_INTERNAL_TOKEN must be at least {MINIMUM_INTERNAL_TOKEN_LENGTH} characters long"
        ),
        Some(token) if token.to_ascii_lowercase().contains("change-me") => {
            anyhow::bail!("NVBES_OBSERVABILITY_INTERNAL_TOKEN cannot use a placeholder value")
        }
        _ => Ok(()),
    }
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

fn parse_sample_rate(value: Option<&str>) -> anyhow::Result<f32> {
    let rate = value
        .unwrap_or("0")
        .parse::<f32>()
        .map_err(|error| anyhow::anyhow!("SENTRY_TRACES_SAMPLE_RATE is invalid: {error}"))?;
    if !(0.0..=1.0).contains(&rate) {
        anyhow::bail!("SENTRY_TRACES_SAMPLE_RATE must be between 0 and 1");
    }
    Ok(rate)
}

#[cfg(test)]
mod tests {
    use super::{ObservabilityConfig, parse_sample_rate, validate};

    fn config() -> ObservabilityConfig {
        ObservabilityConfig {
            sentry_dsn: Some("https://public@example.invalid/1".to_string()),
            sentry_traces_sample_rate: 0.0,
            otlp_endpoint: Some("http://127.0.0.1:4317".to_string()),
            otlp_authorization_header: None,
            observability_internal_token: Some("production-observability-token-value".to_string()),
        }
    }

    #[test]
    fn sentry_trace_sample_rate_is_bounded() {
        assert_eq!(parse_sample_rate(None).unwrap(), 0.0);
        assert_eq!(parse_sample_rate(Some("0.25")).unwrap(), 0.25);
        assert!(parse_sample_rate(Some("-0.1")).is_err());
        assert!(parse_sample_rate(Some("1.1")).is_err());
        assert!(parse_sample_rate(Some("invalid")).is_err());
    }

    #[test]
    fn production_exports_through_alloy_and_protects_metrics() {
        assert!(validate("production", &config()).is_ok());

        let mut missing_sentry = config();
        missing_sentry.sentry_dsn = None;
        assert!(validate("production", &missing_sentry).is_err());

        let mut insecure_sentry = config();
        insecure_sentry.sentry_dsn = Some("http://public@example.invalid/1".to_string());
        assert!(validate("production", &insecure_sentry).is_err());

        let mut direct_export = config();
        direct_export.otlp_authorization_header = Some("Basic secret".to_string());
        assert!(validate("production", &direct_export).is_err());

        let mut public_metrics = config();
        public_metrics.observability_internal_token = None;
        assert!(validate("production", &public_metrics).is_err());
    }
}
