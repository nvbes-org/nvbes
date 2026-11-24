//! Identity services of the API runtime, built once from the environment.
//!
//! Outside `development`/`test` every secret and registry is mandatory and an
//! invalid value stops the process. Development falls back to generated keys
//! and a loopback client so `cargo run` and unit tests need no setup; those
//! fallbacks never apply to a deployed environment.

use std::sync::Arc;

use nvbes_account_service::TokenTrust;
use nvbes_identity_service::{
    browser::BrowserSecurity,
    mfa_crypto::MfaCrypto,
    mfa_keys::{MfaKeyError, MfaKeys},
    oauth::{clients::ClientRegistry, resources::ResourceServers},
    rate_limits::RateLimiter,
    tokens::TokenService,
    tokens_config::TokenConfig,
    webauthn,
};
use webauthn_rs::Webauthn;

use crate::config::{ConfigError, env_lookup, is_development, present};

type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

const DEV_ISSUER: &str = "http://localhost:8080";
const DEV_BROWSER_ORIGIN: &str = "http://localhost:3000";
const DEV_CLIENTS_JSON: &str = r#"[
    {
        "client_id": "nvbes-account-web",
        "display_name": "nvbes Account",
        "redirect_uris": ["http://localhost:3001/oauth/callback"],
        "post_logout_redirect_uris": ["http://localhost:3001/oauth/logout/callback"],
        "resources": {
            "http://localhost:8080": {
                "audience": "nvbes-account-service",
                "scopes": ["account:read", "account:write", "account:export", "account:close"]
            }
        },
        "allow_refresh": true,
        "require_dpop": false,
        "minimum_authentication": "primary"
    }
]"#;

pub struct IdentityRuntime {
    pub tokens: Arc<TokenService>,
    pub trust: TokenTrust,
    pub clients: Arc<ClientRegistry>,
    pub limiter: RateLimiter,
    pub mfa: Arc<MfaCrypto>,
    pub browser: BrowserSecurity,
    pub webauthn: Arc<Webauthn>,
    /// External resource servers allowed to call `/oauth/introspect`. The
    /// in-process Account domain never needs it.
    pub resources: Option<Arc<ResourceServers>>,
}

impl IdentityRuntime {
    pub fn from_env(environment: &str) -> Result<Self, ConfigError> {
        Self::from_lookup(environment, &env_lookup)
    }

    pub fn from_lookup(environment: &str, lookup: Lookup<'_>) -> Result<Self, ConfigError> {
        let development = is_development(environment);
        let optional = |name: &str| present(lookup, name);
        let token_config = token_config(lookup, environment, development)?;
        let trust = TokenTrust {
            issuer: token_config.issuer().to_owned(),
            key_id: token_config.key_id().to_owned(),
            public_key_pem: token_config.public_key_pem().to_owned(),
            verification_keys: token_config
                .verification_keys_json()
                .map_err(|error| ConfigError::Identity(error.to_string()))?,
        };
        let tokens = TokenService::new(token_config)
            .map_err(|error| ConfigError::Identity(error.to_string()))?;

        let clients_json = match optional("NVBES_IDENTITY_OAUTH_CLIENTS_JSON") {
            Some(json) => json,
            None if development => DEV_CLIENTS_JSON.to_owned(),
            None => return Err(ConfigError::Missing("NVBES_IDENTITY_OAUTH_CLIENTS_JSON")),
        };
        let clients = ClientRegistry::from_json(&clients_json, development)
            .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_OAUTH_CLIENTS_JSON"))?;

        let limiter = rate_limiter(lookup, development)?;
        let mfa = mfa_crypto(lookup, development)?;

        let browser_origin = match optional("NVBES_IDENTITY_BROWSER_ORIGIN") {
            Some(origin) => origin,
            None if development => DEV_BROWSER_ORIGIN.to_owned(),
            None => return Err(ConfigError::Missing("NVBES_IDENTITY_BROWSER_ORIGIN")),
        };
        let browser = BrowserSecurity::new(&browser_origin, development)
            .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_BROWSER_ORIGIN"))?;
        let rp_id = reqwest::Url::parse(&browser_origin)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .ok_or(ConfigError::Invalid("NVBES_IDENTITY_BROWSER_ORIGIN"))?;
        let webauthn = webauthn::build_server(&rp_id, &browser_origin)
            .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_BROWSER_ORIGIN"))?;

        let resources = optional("NVBES_IDENTITY_RESOURCE_SERVERS_JSON")
            .map(|json| {
                ResourceServers::from_json(&json)
                    .map(Arc::new)
                    .map_err(|_| ConfigError::Invalid("NVBES_IDENTITY_RESOURCE_SERVERS_JSON"))
            })
            .transpose()?;

        Ok(Self {
            tokens: Arc::new(tokens),
            trust,
            clients: Arc::new(clients),
            limiter,
            mfa: Arc::new(mfa),
            browser,
            webauthn: Arc::new(webauthn),
            resources,
        })
    }
}

/// Development generates signing keys only when no key is configured at all;
/// a partial or invalid configuration is always an error.
fn token_config(
    lookup: Lookup<'_>,
    environment: &str,
    development: bool,
) -> Result<TokenConfig, ConfigError> {
    if development && present(lookup, "NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM").is_none() {
        tracing::info!("generating ephemeral development token signing keys");
        return development_token_config(environment);
    }
    TokenConfig::from_lookup(environment, lookup)
        .map_err(|error| ConfigError::Identity(error.to_string()))
}

fn development_token_config(environment: &str) -> Result<TokenConfig, ConfigError> {
    let identity = |error: String| ConfigError::Identity(error);
    let private = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048)
        .map_err(|error| identity(error.to_string()))?;
    let private_pem = rsa::pkcs8::EncodePrivateKey::to_pkcs8_pem(&private, Default::default())
        .map_err(|error| identity(error.to_string()))?
        .to_string();
    let public_pem = rsa::pkcs8::EncodePublicKey::to_public_key_pem(
        &private.to_public_key(),
        Default::default(),
    )
    .map_err(|error| identity(error.to_string()))?;
    TokenConfig::from_values(
        environment,
        DEV_ISSUER.into(),
        "nvbes-dev-key-1".into(),
        private_pem,
        public_pem,
        nvbes_account_service::ACCOUNT_AUDIENCE.into(),
    )
    .map_err(|error| identity(error.to_string()))
}

fn rate_limiter(lookup: Lookup<'_>, development: bool) -> Result<RateLimiter, ConfigError> {
    const NAME: &str = "NVBES_IDENTITY_RATE_LIMIT_KEY";
    match present(lookup, NAME) {
        Some(key) => RateLimiter::from_base64(&key).map_err(|_| ConfigError::Invalid(NAME)),
        None if development => {
            RateLimiter::new(rand::random::<[u8; 32]>()).map_err(|_| ConfigError::Invalid(NAME))
        }
        None => Err(ConfigError::Missing(NAME)),
    }
}

fn mfa_crypto(lookup: Lookup<'_>, development: bool) -> Result<MfaCrypto, ConfigError> {
    let keys = match MfaKeys::from_lookup(|name| present(lookup, name)) {
        Ok(keys) => keys,
        Err(MfaKeyError::Missing(_)) if development => MfaKeys {
            active_version: 1,
            active_key: rand::random::<[u8; 32]>(),
            previous: None,
        },
        Err(error) => return Err(mfa_error(error)),
    };
    keys.crypto().map_err(mfa_error)
}

fn mfa_error(error: MfaKeyError) -> ConfigError {
    match error {
        MfaKeyError::Missing(name) => ConfigError::Missing(name),
        MfaKeyError::Invalid(name) => ConfigError::Invalid(name),
    }
}

#[cfg(test)]
#[path = "api.identity.runtime.tests.rs"]
mod tests;
