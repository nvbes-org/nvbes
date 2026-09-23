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

pub(crate) fn validate_maxmind_geolite(config: &AppConfig) -> Result<(), String> {
    if !config.maxmind_geolite_database_enabled && !config.maxmind_geolite_web_enabled {
        return Ok(());
    }
    if !config.maxmind_geolite_eula_accepted {
        return Err(
            "NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED=true is required before enabling MaxMind GeoLite"
                .to_string(),
        );
    }
    if config
        .maxmind_account_id
        .as_deref()
        .unwrap_or("")
        .is_empty()
    {
        return Err(
            "NVBES_MAXMIND_ACCOUNT_ID is required when MaxMind GeoLite is enabled".to_string(),
        );
    }
    if config
        .maxmind_license_key
        .as_deref()
        .unwrap_or("")
        .is_empty()
    {
        return Err(
            "NVBES_MAXMIND_LICENSE_KEY is required when MaxMind GeoLite is enabled".to_string(),
        );
    }
    if config.maxmind_web_timeout_secs == 0 {
        return Err("NVBES_MAXMIND_WEB_TIMEOUT_SECS must be greater than zero".to_string());
    }
    if config.maxmind_web_cache_ttl_hours <= 0 {
        return Err("NVBES_MAXMIND_WEB_CACHE_TTL_HOURS must be greater than zero".to_string());
    }
    Ok(())
}

pub(crate) fn validate_loyalsoldier_geoip(config: &AppConfig) -> Result<(), String> {
    if !config.loyalsoldier_geoip_enabled {
        return Ok(());
    }
    if !config.loyalsoldier_geoip_license_accepted {
        return Err(
            "NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED=true is required before enabling Loyalsoldier GeoIP"
                .to_string(),
        );
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
    use crate::config::AppConfig;

    use super::{
        validate_ip_intelligence, validate_loyalsoldier_geoip, validate_maxmind_geolite,
        validate_provider_spec,
    };

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

    #[test]
    fn provider_spec_table_covers_reject_branches() {
        let cases = [
            ("", "must be source|url_template"),
            ("|https://example.test/{ip}", "source must not be empty"),
            (
                "ipinfo|https://example.test/no-ip",
                "url_template must include {ip}",
            ),
            ("ipinfo|not a url/{ip}", "must be a valid URL"),
            (
                "ipinfo|http://example.test/{ip}",
                "must use HTTPS outside development",
            ),
        ];
        for (spec, expected) in cases {
            let err = validate_provider_spec(spec, true).expect_err(spec);
            assert!(
                err.contains(expected),
                "spec={spec:?} err={err} expected={expected}"
            );
        }
        validate_provider_spec("ipinfo|http://example.test/{ip}", false)
            .expect("http allowed in development");
        let host_err = validate_provider_spec("ipinfo|data:text/plain,{ip}", false)
            .expect_err("opaque url without host");
        assert!(host_err.contains("must include a host"), "{host_err}");
    }

    #[test]
    fn ip_intelligence_rejects_non_positive_timeout_and_ttl() {
        let timeout = AppConfig {
            ip_intelligence_timeout_secs: 0,
            ip_intelligence_cache_ttl_hours: 24,
            ..AppConfig::default()
        };
        let err = validate_ip_intelligence(&timeout, false).expect_err("timeout");
        assert!(err.contains("TIMEOUT_SECS"));

        let ttl = AppConfig {
            ip_intelligence_timeout_secs: 2,
            ip_intelligence_cache_ttl_hours: 0,
            ..AppConfig::default()
        };
        let err = validate_ip_intelligence(&ttl, false).expect_err("ttl");
        assert!(err.contains("CACHE_TTL_HOURS"));
    }

    #[test]
    fn ip_intelligence_accepts_valid_provider_list() {
        let config = AppConfig {
            ip_intelligence_provider_specs: vec![
                "ipinfo|https://example.test/{ip}|Bearer token".to_string(),
            ],
            ip_intelligence_timeout_secs: 2,
            ip_intelligence_cache_ttl_hours: 24,
            ..AppConfig::default()
        };
        validate_ip_intelligence(&config, true).expect("valid providers");
    }

    #[test]
    fn maxmind_requires_explicit_eula_acceptance_and_credentials() {
        let config = AppConfig {
            maxmind_geolite_database_enabled: true,
            ..AppConfig::default()
        };
        assert!(validate_maxmind_geolite(&config).is_err());

        let config = AppConfig {
            maxmind_geolite_database_enabled: true,
            maxmind_geolite_eula_accepted: true,
            maxmind_account_id: Some("123".to_string()),
            maxmind_license_key: Some("license".to_string()),
            maxmind_web_timeout_secs: 2,
            maxmind_web_cache_ttl_hours: 24,
            ..AppConfig::default()
        };
        validate_maxmind_geolite(&config).expect("valid maxmind config should pass");
    }

    #[test]
    fn maxmind_table_covers_enabled_credential_and_limits() {
        validate_maxmind_geolite(&AppConfig::default()).expect("disabled");

        let cases: [(&str, AppConfig, &str); 5] = [
            (
                "eula",
                AppConfig {
                    maxmind_geolite_web_enabled: true,
                    ..AppConfig::default()
                },
                "EULA_ACCEPTED",
            ),
            (
                "account",
                AppConfig {
                    maxmind_geolite_web_enabled: true,
                    maxmind_geolite_eula_accepted: true,
                    maxmind_license_key: Some("license".into()),
                    ..AppConfig::default()
                },
                "ACCOUNT_ID",
            ),
            (
                "license",
                AppConfig {
                    maxmind_geolite_database_enabled: true,
                    maxmind_geolite_eula_accepted: true,
                    maxmind_account_id: Some("123".into()),
                    maxmind_license_key: Some(String::new()),
                    ..AppConfig::default()
                },
                "LICENSE_KEY",
            ),
            (
                "timeout",
                AppConfig {
                    maxmind_geolite_database_enabled: true,
                    maxmind_geolite_eula_accepted: true,
                    maxmind_account_id: Some("123".into()),
                    maxmind_license_key: Some("license".into()),
                    maxmind_web_timeout_secs: 0,
                    maxmind_web_cache_ttl_hours: 24,
                    ..AppConfig::default()
                },
                "TIMEOUT_SECS",
            ),
            (
                "ttl",
                AppConfig {
                    maxmind_geolite_database_enabled: true,
                    maxmind_geolite_eula_accepted: true,
                    maxmind_account_id: Some("123".into()),
                    maxmind_license_key: Some("license".into()),
                    maxmind_web_timeout_secs: 2,
                    maxmind_web_cache_ttl_hours: 0,
                    ..AppConfig::default()
                },
                "CACHE_TTL_HOURS",
            ),
        ];
        for (name, config, expected) in cases {
            let err = validate_maxmind_geolite(&config).expect_err(name);
            assert!(
                err.contains(expected),
                "case {name}: err={err} expected={expected}"
            );
        }
    }

    #[test]
    fn loyalsoldier_requires_explicit_license_acceptance() {
        let config = AppConfig {
            loyalsoldier_geoip_enabled: true,
            ..AppConfig::default()
        };
        assert!(validate_loyalsoldier_geoip(&config).is_err());

        let config = AppConfig {
            loyalsoldier_geoip_enabled: true,
            loyalsoldier_geoip_license_accepted: true,
            ..AppConfig::default()
        };
        validate_loyalsoldier_geoip(&config).expect("valid loyalsoldier config should pass");
    }

    #[test]
    fn loyalsoldier_disabled_is_noop() {
        validate_loyalsoldier_geoip(&AppConfig::default()).expect("disabled");
    }
}
