use nvbes_core::security::resource_cors::ResourceCorsOrigins;

/// Account settings that are not owned by Identity. Token trust and token
/// activity come from the Identity domain of the same runtime.
#[derive(Clone, PartialEq)]
pub struct AccountConfig {
    pub browser_origins: ResourceCorsOrigins,
    /// Public origin of the Account API, used as the DPoP `htu` authority.
    pub public_origin: Option<String>,
}

impl AccountConfig {
    /// Outside development both values are mandatory: an empty origin list would
    /// silently lock out the Account web app, and a missing public origin would
    /// reject every DPoP-bound token.
    pub fn from_env(development: bool) -> Result<Self, ConfigError> {
        Self::from_lookup(development, |name| {
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
        })
    }

    /// `lookup` returns the raw value of a variable; blank values count as absent.
    pub fn from_lookup(
        development: bool,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let optional = |name: &str| lookup(name).filter(|value| !value.trim().is_empty());
        let origins_json = match optional("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON") {
            Some(value) => value,
            None if development => "[]".into(),
            None => return Err(ConfigError::Missing("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON")),
        };
        let browser_origins = ResourceCorsOrigins::from_json(&origins_json, development)
            .map_err(|_| ConfigError::Invalid("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON"))?;
        let public_origin = optional("NVBES_ACCOUNT_PUBLIC_ORIGIN");
        if public_origin.is_none() && !development {
            return Err(ConfigError::Missing("NVBES_ACCOUNT_PUBLIC_ORIGIN"));
        }
        Ok(Self {
            browser_origins,
            public_origin,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("required configuration is missing: {0}")]
    Missing(&'static str),
    #[error("configuration is invalid: {0}")]
    Invalid(&'static str),
}

#[cfg(test)]
#[path = "account.config.tests.rs"]
mod tests;
