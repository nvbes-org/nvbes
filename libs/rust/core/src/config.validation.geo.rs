use reqwest::Url;

use crate::config::AppConfig;

pub(crate) fn validate_ip_intelligence(
    config: &AppConfig,
    strict_mode: bool,
) -> Result<(), String> {
    if config.ip_intelligence_timeout_secs == 0 {
        return Err("NVBES_IP_INTELLIGENCE_TIMEOUT_SECS must be greater than zero".to_string());
    }
    if config.ip_intelligence_cache_ttl_hours <= 0 {
        return Err("NVBES_IP_INTELLIGENCE_CACHE_TTL_HOURS must be greater than zero".to_string());
    }

    for spec in &config.ip_intelligence_provider_specs {
        validate_provider_spec(spec, strict_mode)?;
    }

    Ok(())
}

fn validate_provider_spec(spec: &str, strict_mode: bool) -> Result<(), String> {
    let parts = spec.splitn(3, '|').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(
            "NVBES_IP_INTELLIGENCE_PROVIDERS entries must be source|url_template|optional_authorization_header"
                .to_string(),
        );
    }
    if parts[0].is_empty() {
        return Err("NVBES_IP_INTELLIGENCE_PROVIDERS source must not be empty".to_string());
    }
    if !parts[1].contains("{ip}") {
        return Err("NVBES_IP_INTELLIGENCE_PROVIDERS url_template must include {ip}".to_string());
    }

    let url = Url::parse(&parts[1].replace("{ip}", "8.8.8.8"))
        .map_err(|_| "NVBES_IP_INTELLIGENCE_PROVIDERS url_template must be a valid URL")?;
    if strict_mode && url.scheme() != "https" {
        return Err(
            "NVBES_IP_INTELLIGENCE_PROVIDERS url_template must use HTTPS outside development"
                .to_string(),
        );
    }
    url.host_str().ok_or_else(|| {
        "NVBES_IP_INTELLIGENCE_PROVIDERS url_template must include a host".to_string()
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_provider_spec;

    #[test]
    fn validates_provider_spec_shape_and_https() {
        validate_provider_spec("ipinfo|https://example.test/{ip}|Bearer token", true)
            .expect("valid provider spec should pass");
        assert!(validate_provider_spec("broken", false).is_err());
        assert!(
            validate_provider_spec("ipinfo|https://example.test/no-placeholder", false).is_err()
        );
        assert!(validate_provider_spec("ipinfo|http://example.test/{ip}", true).is_err());
    }
}
