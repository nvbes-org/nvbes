use super::{AppConfig, urls};

pub(crate) fn validate_observability_internal_token(
    config: &AppConfig,
    strict_mode: bool,
) -> Result<(), String> {
    let Some(token) = config.observability_internal_token.as_deref() else {
        return if strict_mode {
            Err("NVBES_OBSERVABILITY_INTERNAL_TOKEN is required outside development".to_string())
        } else {
            Ok(())
        };
    };

    if token.len() < 32 {
        return Err(
            "NVBES_OBSERVABILITY_INTERNAL_TOKEN must be at least 32 characters long".to_string(),
        );
    }

    let normalized = token.to_ascii_lowercase();
    if normalized == "default-secret-change-me" || normalized.contains("change-me") {
        return Err(
            "NVBES_OBSERVABILITY_INTERNAL_TOKEN cannot use a placeholder value".to_string(),
        );
    }

    Ok(())
}

pub(crate) fn validate_profiling(config: &AppConfig) -> Result<(), String> {
    if config.profiling_sample_rate_hz == 0 {
        return Err("NVBES_PROFILING_SAMPLE_RATE_HZ must be greater than zero".to_string());
    }
    if !config.profiling_enabled {
        return Ok(());
    }

    let endpoint = config.profiling_endpoint.as_deref().ok_or_else(|| {
        "NVBES_PROFILING_ENDPOINT is required when profiling is enabled".to_string()
    })?;
    urls::validate_profiling_endpoint(endpoint)?;

    match (
        &config.profiling_basic_auth_user,
        &config.profiling_basic_auth_password,
    ) {
        (Some(_), Some(_)) | (None, None) => Ok(()),
        _ => Err(
            "NVBES_PROFILING_BASIC_AUTH_USER and NVBES_PROFILING_BASIC_AUTH_PASSWORD must be set together"
                .to_string(),
        ),
    }
}

pub(crate) fn validate_sentry(config: &AppConfig) -> Result<(), String> {
    if !(0.0..=1.0).contains(&config.sentry_traces_sample_rate) {
        return Err("SENTRY_TRACES_SAMPLE_RATE must be between 0 and 1".to_string());
    }

    Ok(())
}

pub(crate) fn validate_grafana_export_path(
    config: &AppConfig,
    strict_mode: bool,
) -> Result<(), String> {
    if !strict_mode {
        return Ok(());
    }
    if config.otlp_authorization_header.is_some() {
        return Err(
            "NVBES_OTLP_AUTHORIZATION_HEADER must stay empty outside development; export OTLP through local Grafana Alloy"
                .to_string(),
        );
    }
    if config.profiling_basic_auth_user.is_some() || config.profiling_basic_auth_password.is_some()
    {
        return Err(
            "NVBES_PROFILING_BASIC_AUTH_* must stay empty outside development; export profiles through local Grafana Alloy"
                .to_string(),
        );
    }

    Ok(())
}

pub(crate) fn validate_product_analytics(
    config: &AppConfig,
    strict_mode: bool,
) -> Result<(), String> {
    if !config.product_analytics_enabled {
        return Ok(());
    }

    if config
        .product_analytics_token
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err(
            "NVBES_POSTHOG_PROJECT_TOKEN/NVBES_PRODUCT_ANALYTICS_TOKEN is required when NVBES_POSTHOG_ENABLED is true"
                .to_string(),
        );
    }

    let salt = config.analytics_id_salt.as_deref().unwrap_or("").trim();
    if salt.is_empty() {
        return Err(
            "NVBES_ANALYTICS_ID_SALT is required when product analytics is enabled".to_string(),
        );
    }
    if strict_mode && salt.len() < 32 {
        return Err(
            "NVBES_ANALYTICS_ID_SALT must be at least 32 characters outside development"
                .to_string(),
        );
    }

    urls::validate_public_url(
        "NVBES_POSTHOG_HOST",
        &config.posthog_host,
        strict_mode,
        true,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_grafana_export_path, validate_observability_internal_token,
        validate_product_analytics, validate_profiling, validate_sentry,
    };
    use crate::config::AppConfig;

    #[test]
    fn observability_token_table_covers_presence_and_strength() {
        validate_observability_internal_token(&AppConfig::default(), false)
            .expect("optional in development");

        let cases = [
            (None, true, "required outside development"),
            (Some("short"), false, "at least 32 characters"),
            (
                Some("default-secret-change-me-xxxxxxxxxxxx"),
                true,
                "placeholder",
            ),
            (Some("a-very-long-observability-token-value"), true, "ok"),
        ];
        for (token, strict, expected) in cases {
            let config = AppConfig {
                observability_internal_token: token.map(str::to_string),
                ..AppConfig::default()
            };
            let result = validate_observability_internal_token(&config, strict);
            if expected == "ok" {
                result.expect("valid token");
            } else {
                let err = result.expect_err(expected);
                assert!(err.contains(expected), "err={err} expected={expected}");
            }
        }
    }

    #[test]
    fn profiling_table_covers_rate_endpoint_and_auth_pairing() {
        let zero_rate = AppConfig {
            profiling_sample_rate_hz: 0,
            ..AppConfig::default()
        };
        assert!(
            validate_profiling(&zero_rate)
                .unwrap_err()
                .contains("SAMPLE_RATE_HZ")
        );

        validate_profiling(&AppConfig {
            profiling_enabled: false,
            profiling_sample_rate_hz: 100,
            ..AppConfig::default()
        })
        .expect("disabled profiling");

        let enabled_ok = AppConfig {
            profiling_enabled: true,
            profiling_sample_rate_hz: 100,
            profiling_endpoint: Some("http://127.0.0.1:4040".into()),
            profiling_basic_auth_user: Some("u".into()),
            profiling_basic_auth_password: Some("p".into()),
            ..AppConfig::default()
        };
        validate_profiling(&enabled_ok).expect("paired auth");

        let password_only = AppConfig {
            profiling_enabled: true,
            profiling_sample_rate_hz: 100,
            profiling_endpoint: Some("http://127.0.0.1:4040".into()),
            profiling_basic_auth_password: Some("p".into()),
            ..AppConfig::default()
        };
        assert!(
            validate_profiling(&password_only)
                .unwrap_err()
                .contains("must be set together")
        );
    }

    #[test]
    fn sentry_accepts_boundary_sample_rates() {
        for rate in [0.0, 0.5, 1.0] {
            let config = AppConfig {
                sentry_traces_sample_rate: rate,
                ..AppConfig::default()
            };
            validate_sentry(&config).expect("valid rate");
        }
        let invalid = AppConfig {
            sentry_traces_sample_rate: -0.1,
            ..AppConfig::default()
        };
        assert!(validate_sentry(&invalid).is_err());
    }

    #[test]
    fn grafana_export_path_is_noop_outside_strict_mode() {
        let config = AppConfig {
            otlp_authorization_header: Some("Basic x".into()),
            profiling_basic_auth_user: Some("u".into()),
            ..AppConfig::default()
        };
        validate_grafana_export_path(&config, false).expect("non-strict");
        validate_grafana_export_path(&AppConfig::default(), true).expect("empty strict");
    }

    #[test]
    fn product_analytics_table_covers_token_salt_and_disabled() {
        validate_product_analytics(&AppConfig::default(), true).expect("disabled");

        let missing_salt = AppConfig {
            product_analytics_enabled: true,
            product_analytics_token: Some("token".into()),
            analytics_id_salt: Some("   ".into()),
            posthog_host: "https://us.i.posthog.com".into(),
            ..AppConfig::default()
        };
        assert!(
            validate_product_analytics(&missing_salt, false)
                .unwrap_err()
                .contains("ANALYTICS_ID_SALT")
        );

        let ok = AppConfig {
            product_analytics_enabled: true,
            product_analytics_token: Some("token".into()),
            analytics_id_salt: Some("a".repeat(32)),
            posthog_host: "https://us.i.posthog.com".into(),
            ..AppConfig::default()
        };
        validate_product_analytics(&ok, true).expect("valid analytics");
    }
}
