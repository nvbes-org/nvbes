use std::time::Duration;

use url::Url;

#[derive(Clone)]
pub struct AccountWorkerConfig {
    pub database_url: String,
    pub database_max_connections: u32,
    pub identity_service_base_url: String,
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub poll_interval: Duration,
    pub retry_interval: Duration,
    pub claim_timeout: Duration,
    pub max_attempts: i32,
    pub avatar_storage: AvatarStorageConfig,
}

#[derive(Clone)]
pub enum AvatarStorageConfig {
    Mock,
    S3 {
        bucket: String,
        endpoint: String,
        region: String,
        access_key: String,
        secret_key: String,
    },
}

impl AccountWorkerConfig {
    pub fn from_env() -> Result<Self, String> {
        let environment =
            std::env::var("NVBES_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let poll_interval =
            Duration::from_millis(optional_parse("NVBES_ACCOUNT_WORKER_POLL_INTERVAL_MS", 1_000)?);
        let retry_interval =
            Duration::from_secs(optional_parse("NVBES_ACCOUNT_WORKER_RETRY_INTERVAL_SECONDS", 60)?);
        let claim_timeout =
            Duration::from_secs(optional_parse("NVBES_ACCOUNT_WORKER_CLAIM_TIMEOUT_SECONDS", 300)?);
        let max_attempts = optional_parse("NVBES_ACCOUNT_WORKER_MAX_ATTEMPTS", 10)?;

        if poll_interval.is_zero() || retry_interval.is_zero() || claim_timeout.is_zero() {
            return Err("Account worker intervals must be greater than zero".to_string());
        }
        if max_attempts < 1 {
            return Err("NVBES_ACCOUNT_WORKER_MAX_ATTEMPTS must be at least 1".to_string());
        }

        Ok(Self {
            database_url: required("NVBES_ACCOUNT_DATABASE_URL")?,
            database_max_connections: optional_parse(
                "NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS",
                10,
            )?,
            identity_service_base_url: absolute_http_base_url(
                "NVBES_IDENTITY_SERVICE_BASE_URL",
                required("NVBES_IDENTITY_SERVICE_BASE_URL")?,
            )?,
            oauth_client_id: required("NVBES_ACCOUNT_WORKER_OAUTH_CLIENT_ID")?,
            oauth_client_secret: required("NVBES_ACCOUNT_WORKER_OAUTH_CLIENT_SECRET")?,
            poll_interval,
            retry_interval,
            claim_timeout,
            max_attempts,
            avatar_storage: avatar_storage_from_env(&environment)?,
        })
    }
}

fn avatar_storage_from_env(environment: &str) -> Result<AvatarStorageConfig, String> {
    match required("NVBES_ACCOUNT_AVATAR_STORAGE_MODE")?.as_str() {
        "mock" if matches!(environment, "development" | "test") => Ok(AvatarStorageConfig::Mock),
        "mock" => Err(
            "NVBES_ACCOUNT_AVATAR_STORAGE_MODE=mock is forbidden outside development/test"
                .to_string(),
        ),
        "s3" => Ok(AvatarStorageConfig::S3 {
            bucket: required("NVBES_ACCOUNT_AVATAR_STORAGE_BUCKET")?,
            endpoint: absolute_http_base_url(
                "NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT",
                required("NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT")?,
            )?,
            region: required("NVBES_ACCOUNT_AVATAR_STORAGE_REGION")?,
            access_key: required("NVBES_ACCOUNT_AVATAR_STORAGE_ACCESS_KEY")?,
            secret_key: required("NVBES_ACCOUNT_AVATAR_STORAGE_SECRET_KEY")?,
        }),
        value => Err(format!(
            "NVBES_ACCOUNT_AVATAR_STORAGE_MODE must be `s3` or `mock`, got `{value}`"
        )),
    }
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
    fn identity_base_url_rejects_credentials_queries_and_fragments() {
        for value in [
            "https://user@example.test",
            "https://example.test?debug=1",
            "https://example.test#fragment",
        ] {
            assert!(absolute_http_base_url("URL", value.to_string()).is_err());
        }
    }

    #[test]
    fn identity_base_url_is_normalized() {
        assert_eq!(
            absolute_http_base_url("URL", "https://identity.example.test/".to_string())
                .expect("valid URL"),
            "https://identity.example.test"
        );
    }
}
