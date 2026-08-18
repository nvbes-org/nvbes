use url::Url;

#[derive(Clone)]
pub struct AccountConfig {
    pub database_url: String,
    pub database_max_connections: u32,
    pub service_port: u16,
    pub service_base_url: String,
    pub identity_service_base_url: String,
    pub account_web_base_url: String,
    pub account_web_origin: String,
    pub provisioning_token: String,
    pub environment: String,
    pub avatar_storage: AvatarStorageConfig,
}

#[derive(Clone)]
pub enum AvatarStorageConfig {
    Mock,
    S3 {
        bucket: String,
        endpoint: String,
        public_endpoint: Option<String>,
        region: String,
        access_key: String,
        secret_key: String,
    },
}

impl AccountConfig {
    pub fn from_env() -> Result<Self, String> {
        let service_base_url = required_url(
            "NVBES_ACCOUNT_SERVICE_BASE_URL",
            required("NVBES_ACCOUNT_SERVICE_BASE_URL")?,
        )?;
        let identity_service_base_url = required_url(
            "NVBES_IDENTITY_SERVICE_BASE_URL",
            required("NVBES_IDENTITY_SERVICE_BASE_URL")?,
        )?;
        let account_web_base_url = required_url(
            "NVBES_ACCOUNT_WEB_BASE_URL",
            required("NVBES_ACCOUNT_WEB_BASE_URL")?,
        )?;
        let account_web_origin = origin(&account_web_base_url)?;
        let environment =
            std::env::var("NVBES_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let avatar_storage = avatar_storage_from_env(&environment)?;

        Ok(Self {
            database_url: required("NVBES_ACCOUNT_DATABASE_URL")?,
            database_max_connections: optional_parse("NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS", 20)?,
            service_port: required_parse("NVBES_ACCOUNT_SERVICE_PORT")?,
            service_base_url,
            identity_service_base_url,
            account_web_base_url,
            account_web_origin,
            provisioning_token: provisioning_token(&environment)?,
            environment,
            avatar_storage,
        })
    }
}

fn provisioning_token(environment: &str) -> Result<String, String> {
    match std::env::var("NVBES_ACCOUNT_PROVISIONING_TOKEN") {
        Ok(value) if value.trim().len() >= 32 => Ok(value.trim().to_string()),
        Ok(_) => Err("NVBES_ACCOUNT_PROVISIONING_TOKEN must contain at least 32 characters".into()),
        Err(std::env::VarError::NotPresent) if matches!(environment, "development" | "test") => {
            Ok("development-account-provisioning-token".to_string())
        }
        Err(std::env::VarError::NotPresent) => {
            Err("NVBES_ACCOUNT_PROVISIONING_TOKEN is required".to_string())
        }
        Err(error) => Err(format!(
            "NVBES_ACCOUNT_PROVISIONING_TOKEN could not be read: {error}"
        )),
    }
}

pub fn avatar_storage_from_env(environment: &str) -> Result<AvatarStorageConfig, String> {
    match required("NVBES_ACCOUNT_AVATAR_STORAGE_MODE")?.as_str() {
        "mock" if matches!(environment, "development" | "test") => Ok(AvatarStorageConfig::Mock),
        "mock" => Err(
            "NVBES_ACCOUNT_AVATAR_STORAGE_MODE=mock is forbidden outside development/test"
                .to_string(),
        ),
        "s3" => Ok(AvatarStorageConfig::S3 {
            bucket: required("NVBES_ACCOUNT_AVATAR_STORAGE_BUCKET")?,
            endpoint: required_url(
                "NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT",
                required("NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT")?,
            )?,
            public_endpoint: optional_url("NVBES_ACCOUNT_AVATAR_STORAGE_PUBLIC_ENDPOINT")?,
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

fn required_parse<T>(name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    required(name)?
        .parse()
        .map_err(|error| format!("{name} is invalid: {error}"))
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

fn optional_url(name: &str) -> Result<Option<String>, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| required_url(name, value))
        .transpose()
}

fn required_url(name: &str, value: String) -> Result<String, String> {
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

fn origin(base_url: &str) -> Result<String, String> {
    let parsed = Url::parse(base_url).map_err(|error| error.to_string())?;
    if parsed.path() != "/" {
        return Err("NVBES_ACCOUNT_WEB_BASE_URL must not contain a path".to_string());
    }
    Ok(parsed.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::{origin, provisioning_token, required_url};

    #[test]
    fn account_web_origin_is_exact() {
        assert_eq!(
            origin("https://account.example").expect("valid origin"),
            "https://account.example"
        );
        assert!(origin("https://account.example/app").is_err());
    }

    #[test]
    fn base_urls_reject_credentials_and_queries() {
        assert!(required_url("URL", "https://user@example.com".to_string()).is_err());
        assert!(required_url("URL", "https://example.com?debug=1".to_string()).is_err());
    }

    #[test]
    fn provisioning_token_is_strong_or_development_only() {
        assert!(provisioning_token("development").is_ok());
        assert!(provisioning_token("test").is_ok());
    }
}
