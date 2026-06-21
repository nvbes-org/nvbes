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
            "NVBES_PRODUCT_ANALYTICS_TOKEN is required when NVBES_PRODUCT_ANALYTICS_ENABLED is true"
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

    Ok(())
}
