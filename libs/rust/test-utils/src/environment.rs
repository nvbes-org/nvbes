use std::net::IpAddr;

const ALLOWED_ENVIRONMENTS: &[&str] = &[
    "ci",
    "dev",
    "development",
    "integration",
    "local",
    "test",
    "testing",
];

/// Validates that `NVBES_ENV` is safe for test execution.
///
/// Returns `Ok(())` when the environment is unset, empty, or matches one of the
/// allowed test contexts. Returns `Err` for `staging`, `production`, or any
/// unexpected value.
pub fn validate_test_environment(environment: Option<&str>) -> Result<(), String> {
    let Some(environment) = environment.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if ALLOWED_ENVIRONMENTS
        .iter()
        .any(|allowed| allowed == &environment.to_ascii_lowercase().as_str())
    {
        return Ok(());
    }
    Err(format!(
        "tests require a local/test environment; refusing NVBES_ENV={environment:?}"
    ))
}

/// Validates that a URL points to a loopback or local Docker host.
///
/// Accepts `localhost`, loopback IPs (127.x.x.x, ::1), and `host.docker.internal`.
/// Returns `Err` for any remote or potentially deceptive host.
pub fn validate_loopback_url(url_str: &str, resource: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(url_str).map_err(|_| format!("{resource} URL is invalid"))?;
    let host = url
        .host_str()
        .ok_or_else(|| format!("{resource} URL must contain a host"))?;
    let normalized = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    let is_loopback = normalized.eq_ignore_ascii_case("localhost")
        || normalized
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    let is_local_docker_host = normalized.eq_ignore_ascii_case("host.docker.internal");
    if is_loopback || is_local_docker_host {
        Ok(())
    } else {
        Err(format!(
            "integration tests only accept loopback {resource}; refusing host {normalized:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_test_environment_accepts_none() {
        assert!(validate_test_environment(None).is_ok());
    }

    #[test]
    fn validate_test_environment_accepts_test_contexts() {
        for env in [
            "ci",
            "dev",
            "development",
            "integration",
            "local",
            "test",
            "testing",
        ] {
            assert!(validate_test_environment(Some(env)).is_ok(), "{env}");
        }
    }

    #[test]
    fn validate_test_environment_rejects_production() {
        for env in ["staging", "production", "prod"] {
            assert!(validate_test_environment(Some(env)).is_err(), "{env}");
        }
    }

    #[test]
    fn validate_loopback_url_accepts_localhost() {
        assert!(validate_loopback_url("redis://localhost:6379", "Redis").is_ok());
    }

    #[test]
    fn validate_loopback_url_accepts_loopback_ip() {
        assert!(validate_loopback_url("redis://127.0.0.1:6379", "Redis").is_ok());
        assert!(validate_loopback_url("redis://127.42.0.1:16379/15", "Redis").is_ok());
    }

    #[test]
    fn validate_loopback_url_accepts_docker_host() {
        assert!(validate_loopback_url("redis://host.docker.internal:6379/15", "Redis").is_ok());
    }

    #[test]
    fn validate_loopback_url_rejects_remote_hosts() {
        for url in ["redis://redis.internal:6379", "redis://10.0.0.1:6379"] {
            assert!(validate_loopback_url(url, "Redis").is_err(), "{url}");
        }
    }

    #[test]
    fn parses_ipv6_and_rejects_deceptive_authorities_without_leaking_credentials() {
        assert!(validate_loopback_url("redis://[::1]:6379", "Redis").is_ok());
        for url in [
            "redis://localhost@remote.example:6379",
            "redis://localhost.evil:6379",
            "not-a-url",
        ] {
            assert!(validate_loopback_url(url, "Redis").is_err());
        }
        let error =
            validate_loopback_url("redis://user:secret@remote.example:6379", "Redis").unwrap_err();
        assert!(!error.contains("secret"));
    }
}
