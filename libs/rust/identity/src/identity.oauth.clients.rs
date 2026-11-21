use std::collections::{BTreeMap, BTreeSet};

use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::error::OAuthError;

const MAX_CLIENTS: usize = 32;
const MAX_REDIRECTS: usize = 8;
const MAX_CONFIGURATION_BYTES: usize = 65_536;

/// Operator-managed public clients. No browser may register or widen its own policy.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicClient {
    pub client_id: String,
    pub display_name: String,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
    pub resources: BTreeMap<String, ResourcePolicy>,
    pub allow_refresh: bool,
    pub require_dpop: bool,
    #[serde(default)]
    pub minimum_authentication: super::authentication_policy::AuthenticationPolicy,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePolicy {
    pub audience: String,
    pub scopes: Vec<String>,
}

#[derive(Clone)]
pub struct ClientRegistry {
    clients: BTreeMap<String, PublicClient>,
}

impl ClientRegistry {
    pub fn from_json(input: &str, development: bool) -> Result<Self, OAuthError> {
        if input.len() > MAX_CONFIGURATION_BYTES {
            return Err(OAuthError::InvalidClient);
        }
        let clients: Vec<PublicClient> =
            serde_json::from_str(input).map_err(|_| OAuthError::InvalidClient)?;
        if clients.is_empty() || clients.len() > MAX_CLIENTS {
            return Err(OAuthError::InvalidClient);
        }
        let mut registry = BTreeMap::new();
        for client in clients {
            validate_client(&client, development)?;
            if registry.insert(client.client_id.clone(), client).is_some() {
                return Err(OAuthError::InvalidClient);
            }
        }
        Ok(Self { clients: registry })
    }

    pub fn get(&self, id: &str) -> Result<&PublicClient, OAuthError> {
        self.clients.get(id).ok_or(OAuthError::InvalidClient)
    }

    /// Exact web origins from validated redirects; no wildcard or suffix matching.
    pub fn browser_origins(&self) -> BTreeSet<String> {
        self.clients
            .values()
            .flat_map(|client| &client.redirect_uris)
            .filter_map(|uri| Url::parse(uri).ok())
            .map(|url| url.origin().ascii_serialization())
            .collect()
    }
}

fn validate_client(client: &PublicClient, development: bool) -> Result<(), OAuthError> {
    if !(3..=128).contains(&client.client_id.len())
        || !client
            .client_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || client.display_name.trim().is_empty()
        || client.display_name.len() > 100
        || client.display_name.chars().any(char::is_control)
        || client.resources.is_empty()
        || client.resources.len() > 3
    {
        return Err(OAuthError::InvalidClient);
    }
    validate_redirects(&client.redirect_uris, development, true)?;
    validate_redirects(&client.post_logout_redirect_uris, development, false)?;
    for (resource, policy) in &client.resources {
        validate_web_url(resource, development)?;
        let scopes = &policy.scopes;
        let allowed: &[&str] = match policy.audience.as_str() {
            "nvbes-account-service" => &[
                "account:read",
                "account:write",
                "account:export",
                "account:close",
            ],
            "nvbes-billing-service" => &["billing:read", "billing:checkout"],
            "nvbes-identity-userinfo" => &["openid", "profile", "email"],
            _ => return Err(OAuthError::InvalidTarget),
        };
        if scopes.is_empty()
            || (policy.audience == crate::tokens_policy::USERINFO_AUDIENCE
                && !scopes.iter().any(|s| s == "openid"))
            || scopes.len() > allowed.len()
            || scopes.iter().collect::<BTreeSet<_>>().len() != scopes.len()
            || scopes.iter().any(|s| !allowed.contains(&s.as_str()))
        {
            return Err(OAuthError::InvalidScope);
        }
    }
    Ok(())
}

fn validate_redirects(
    uris: &[String],
    development: bool,
    required: bool,
) -> Result<(), OAuthError> {
    if (required && uris.is_empty())
        || uris.len() > MAX_REDIRECTS
        || uris.iter().collect::<BTreeSet<_>>().len() != uris.len()
    {
        return Err(OAuthError::InvalidClient);
    }
    for uri in uris {
        validate_web_url(uri, development)?;
    }
    Ok(())
}

/// Initial web profile: HTTPS, or explicit loopback HTTP only for local tests/dev.
/// Native redirect profiles will be registered separately when those clients ship.
pub fn validate_web_url(value: &str, development: bool) -> Result<Url, OAuthError> {
    if value.len() > 2048 || value.contains(['*', '\\']) || value.chars().any(char::is_control) {
        return Err(OAuthError::InvalidClient);
    }
    let url = Url::parse(value).map_err(|_| OAuthError::InvalidClient)?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (development && loopback && url.scheme() == "http"))
    {
        return Err(OAuthError::InvalidClient);
    }
    Ok(url)
}
