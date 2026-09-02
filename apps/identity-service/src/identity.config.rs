use base64::{Engine, engine::general_purpose::STANDARD};
use std::net::SocketAddr;

const DEVELOPMENT_MFA_KEY: &str = "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityConfig {
    pub environment: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub bind_addr: SocketAddr,
    pub mfa_encryption_key: [u8; 32],
    pub mfa_key_version: i16,
    pub mfa_previous_encryption_key: Option<[u8; 32]>,
    pub mfa_previous_key_version: Option<i16>,
}

impl IdentityConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
        let database_url = database_url(&environment)?;
        let database_max_connections = optional("NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|| "5".into())
            .parse::<u32>()
            .ok()
            .filter(|value| (1..=20).contains(value))
            .ok_or(ConfigError::Invalid(
                "NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS",
            ))?;
        let default_bind_addr = std::env::var("PORT")
            .map(|port| format!("0.0.0.0:{port}"))
            .unwrap_or_else(|_| "127.0.0.1:3060".into());
        let bind_addr = optional("NVBES_IDENTITY_BIND_ADDR")
            .unwrap_or(default_bind_addr)
            .parse()
            .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_BIND_ADDR"))?;
        let mfa_encryption_key = optional("NVBES_IDENTITY_MFA_ENCRYPTION_KEY")
            .or_else(|| {
                matches!(environment.as_str(), "development" | "test")
                    .then(|| DEVELOPMENT_MFA_KEY.into())
            })
            .ok_or(ConfigError::Missing("NVBES_IDENTITY_MFA_ENCRYPTION_KEY"))
            .and_then(|value| decode_key(&value))?;
        let mfa_key_version = version(
            "NVBES_IDENTITY_MFA_KEY_VERSION",
            matches!(environment.as_str(), "development" | "test").then_some(1),
        )?;
        let previous_key = optional("NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY");
        let previous_version = optional("NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION");
        let (mfa_previous_encryption_key, mfa_previous_key_version) =
            match (previous_key, previous_version) {
                (None, None) => (None, None),
                (Some(key), Some(version_value)) => {
                    let version =
                        parse_version("NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION", &version_value)?;
                    if version == mfa_key_version {
                        return Err(ConfigError::Invalid("MFA key versions must be distinct"));
                    }
                    (Some(decode_key(&key)?), Some(version))
                }
                _ => return Err(ConfigError::Invalid("MFA previous key pair")),
            };

        Ok(Self {
            environment,
            database_url,
            database_max_connections,
            bind_addr,
            mfa_encryption_key,
            mfa_key_version,
            mfa_previous_encryption_key,
            mfa_previous_key_version,
        })
    }
}

fn decode_key(value: &str) -> Result<[u8; 32], ConfigError> {
    let decoded = STANDARD
        .decode(value)
        .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_MFA_ENCRYPTION_KEY"))?;
    decoded
        .try_into()
        .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_MFA_ENCRYPTION_KEY"))
}

fn version(name: &'static str, default: Option<i16>) -> Result<i16, ConfigError> {
    match optional(name) {
        Some(value) => parse_version(name, &value),
        None => default.ok_or(ConfigError::Missing(name)),
    }
}

fn parse_version(name: &'static str, value: &str) -> Result<i16, ConfigError> {
    value
        .parse::<i16>()
        .ok()
        .filter(|version| *version > 0)
        .ok_or(ConfigError::Invalid(name))
}

pub fn database_url_from_env() -> Result<String, ConfigError> {
    let environment = optional("NVBES_ENVIRONMENT").unwrap_or_else(|| "development".into());
    database_url(&environment)
}

fn database_url(environment: &str) -> Result<String, ConfigError> {
    optional("NVBES_IDENTITY_DATABASE_URL")
        .or_else(|| {
            matches!(environment, "development" | "test")
                .then(|| "postgres://localhost/nvbes_identity".into())
        })
        .ok_or(ConfigError::Missing("NVBES_IDENTITY_DATABASE_URL"))
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("required configuration is missing: {0}")]
    Missing(&'static str),
    #[error("configuration is invalid: {0}")]
    Invalid(&'static str),
}

#[cfg(test)]
#[path = "identity.config.tests.rs"]
mod tests;
