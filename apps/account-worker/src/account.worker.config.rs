use std::time::Duration;

use url::Url;

#[derive(Clone)]
pub struct AccountWorkerConfig {
    pub database_url: String,
    pub database_max_connections: u32,
    pub identity_service_base_url: String,
    pub identity_internal_token: String,
    pub cloud_service_base_url: String,
    pub cloud_internal_token: String,
    pub billing_service_base_url: String,
    pub billing_internal_token: String,
    pub avatar_storage: nvbes_account_service::config::AvatarStorageConfig,
    pub poll_interval: Duration,
    pub claim_timeout: Duration,
    pub max_attempts: i32,
}

impl AccountWorkerConfig {
    pub fn from_env() -> Result<Self, String> {
        let environment =
            std::env::var("NVBES_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let poll_interval = Duration::from_millis(optional_parse(
            "NVBES_ACCOUNT_WORKER_POLL_INTERVAL_MS",
            1_000,
        )?);
        let claim_timeout = Duration::from_secs(optional_parse(
            "NVBES_ACCOUNT_WORKER_CLAIM_TIMEOUT_SECONDS",
            300,
        )?);
        let max_attempts = optional_parse("NVBES_ACCOUNT_WORKER_MAX_ATTEMPTS", 12)?;
        if poll_interval.is_zero() || claim_timeout.is_zero() {
            return Err("Account worker intervals must be greater than zero".to_string());
        }
        if max_attempts < 1 {
            return Err("NVBES_ACCOUNT_WORKER_MAX_ATTEMPTS must be at least 1".to_string());
        }

        Ok(Self {
            database_url: required("NVBES_ACCOUNT_DATABASE_URL")?,
            database_max_connections: optional_parse("NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS", 10)?,
            identity_service_base_url: absolute_http_base_url(
                "NVBES_IDENTITY_SERVICE_BASE_URL",
                required("NVBES_IDENTITY_SERVICE_BASE_URL")?,
            )?,
            identity_internal_token: nvbes_core::http::internal_service::load_token(
                "NVBES_IDENTITY_INTERNAL_TOKEN",
                &environment,
                "development-identity-internal-token",
            )?,
            cloud_service_base_url: service_base_url(
                "NVBES_CLOUD_SERVICE_BASE_URL",
                &environment,
                "http://localhost:4002",
            )?,
            cloud_internal_token: nvbes_core::http::internal_service::load_token(
                "NVBES_CLOUD_INTERNAL_TOKEN",
                &environment,
                "development-cloud-internal-token-01",
            )?,
            billing_service_base_url: service_base_url(
                "NVBES_BILLING_SERVICE_BASE_URL",
                &environment,
                "http://localhost:4020",
            )?,
            billing_internal_token: nvbes_core::http::internal_service::load_token(
                "NVBES_BILLING_INTERNAL_TOKEN",
                &environment,
                "development-billing-internal-token",
            )?,
            avatar_storage: nvbes_account_service::config::avatar_storage_from_env(&environment)?,
            poll_interval,
            claim_timeout,
            max_attempts,
        })
    }
}

fn service_base_url(
    name: &str,
    environment: &str,
    development_default: &str,
) -> Result<String, String> {
    let value = match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        Ok(_) => return Err(format!("{name} cannot be empty")),
        Err(std::env::VarError::NotPresent) if matches!(environment, "development" | "test") => {
            development_default.to_string()
        }
        Err(std::env::VarError::NotPresent) => return Err(format!("{name} is required")),
        Err(error) => return Err(format!("{name} could not be read: {error}")),
    };
    absolute_http_base_url(name, value)
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} is required"))
}

fn optional_parse<T>(name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|error| format!("{name} is invalid: {error}")),
        Err(_) => Ok(default),
    }
}

fn absolute_http_base_url(name: &str, value: String) -> Result<String, String> {
    let parsed = Url::parse(value.trim()).map_err(|error| format!("{name} is invalid: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(format!("{name} must be an absolute HTTP(S) base URL"));
    }
    Ok(value.trim().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::absolute_http_base_url;

    #[test]
    fn identity_base_url_rejects_ambiguous_urls() {
        for value in [
            "https://user@example.test",
            "https://example.test?debug=1",
            "https://example.test#fragment",
        ] {
            assert!(absolute_http_base_url("URL", value.to_string()).is_err());
        }
    }
}
