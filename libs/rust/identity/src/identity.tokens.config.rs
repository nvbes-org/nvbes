use crate::tokens_error::TokenError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Private signing material must never be formatted into logs.
#[derive(Clone)]
pub struct TokenConfig {
    pub(crate) issuer: String,
    pub(crate) key_id: String,
    pub(crate) private_key_pem: String,
    pub(crate) public_key_pem: String,
    pub(crate) allowed_audiences: BTreeSet<String>,
    pub(crate) verification_keys: Vec<VerificationKeyConfig>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VerificationKeyConfig {
    pub kid: String,
    pub public_key_pem: String,
    /// Absolute UTC timestamp. Retired keys disappear from JWKS and verification.
    pub accept_until: u64,
}

impl TokenConfig {
    pub fn from_env(environment: &str) -> Result<Self, TokenError> {
        // Non-UTF-8 values stay present (lossily decoded) so they fail validation
        // instead of being mistaken for an absent variable.
        Self::from_lookup(environment, |name| {
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
        })
    }

    /// `lookup` returns the raw value of a variable; blank values count as absent.
    pub fn from_lookup(
        environment: &str,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, TokenError> {
        let required = |name: &'static str| {
            lookup(name)
                .filter(|value| !value.trim().is_empty())
                .ok_or(TokenError::Configuration(name))
        };
        let config = Self::from_values(
            environment,
            required("NVBES_IDENTITY_TOKEN_ISSUER")?,
            required("NVBES_IDENTITY_TOKEN_KEY_ID")?,
            required("NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM")?,
            required("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM")?,
            required("NVBES_IDENTITY_TOKEN_AUDIENCES")?,
        )?;
        match lookup("NVBES_IDENTITY_TOKEN_VERIFICATION_KEYS") {
            Some(keys) => config.with_verification_keys(&keys),
            None => Ok(config),
        }
    }

    pub fn from_values(
        environment: &str,
        issuer: String,
        key_id: String,
        private_key_pem: String,
        public_key_pem: String,
        audiences: String,
    ) -> Result<Self, TokenError> {
        let parsed =
            reqwest::Url::parse(&issuer).map_err(|_| TokenError::Configuration("issuer URL"))?;
        let local_http = matches!(environment, "development" | "test")
            && parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if parsed.host_str().is_none()
            || !(parsed.scheme() == "https" || local_http)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
            || issuer.contains('\\')
            || issuer.bytes().any(|b| b.is_ascii_control() || b == b' ')
        {
            return Err(TokenError::Configuration("issuer must be an HTTPS origin"));
        }
        // Preserve the configured issuer exactly; OIDC comparisons are exact.
        validate_identifier(&key_id)?;
        let allowed_audiences = audiences
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if allowed_audiences.is_empty() {
            return Err(TokenError::Configuration("missing token audiences"));
        }
        for audience in &allowed_audiences {
            if !matches!(
                audience.as_str(),
                "nvbes-account-service" | "nvbes-billing-service" | "nvbes-identity-userinfo"
            ) {
                return Err(TokenError::Configuration("unknown token audience"));
            }
        }
        Ok(Self {
            issuer,
            key_id,
            private_key_pem,
            public_key_pem,
            allowed_audiences,
            verification_keys: Vec::new(),
        })
    }

    /// Prepublish a new public key, or retain an old public key during rotation.
    /// Only the active private key signs. At most three additional keys overlap.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn public_key_pem(&self) -> &str {
        &self.public_key_pem
    }

    /// Overlap keys in the same JSON shape resource servers pin, so in-process
    /// verifiers trust exactly the keys this issuer still accepts.
    pub fn verification_keys_json(&self) -> Result<String, TokenError> {
        serde_json::to_string(&self.verification_keys)
            .map_err(|_| TokenError::Configuration("verification key encoding"))
    }

    pub fn with_verification_keys(mut self, json: &str) -> Result<Self, TokenError> {
        if json.len() > 32_768 {
            return Err(TokenError::Configuration("verification keys too large"));
        }
        let keys: Vec<VerificationKeyConfig> = serde_json::from_str(json)
            .map_err(|_| TokenError::Configuration("verification key schema"))?;
        let mut ids = BTreeSet::from([self.key_id.as_str()]);
        if keys.len() > 3 {
            return Err(TokenError::Configuration("too many verification keys"));
        }
        for key in &keys {
            validate_identifier(&key.kid)?;
            if !ids.insert(&key.kid) || key.accept_until == 0 {
                return Err(TokenError::Configuration(
                    "duplicate or invalid verification key",
                ));
            }
        }
        self.verification_keys = keys;
        Ok(self)
    }
}

fn validate_identifier(value: &str) -> Result<(), TokenError> {
    if !(3..=128).contains(&value.len())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':' | b'.'))
    {
        return Err(TokenError::Configuration("key ID"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "identity.tokens.config.tests.rs"]
mod tests;
