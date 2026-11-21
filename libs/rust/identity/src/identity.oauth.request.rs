use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{clients::ClientRegistry, error::OAuthError, pkce};

/// Input shared by direct authorization and PAR. Duplicate known form fields
/// must be rejected by the HTTP decoder before validation.
#[derive(Deserialize)]
pub struct AuthorizationInput {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub resource: String,
    pub state: String,
    pub nonce: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub dpop_jkt: Option<String>,
    pub max_age: Option<u32>,
    pub prompt: Option<String>,
}

/// Only constructed after the registered redirect, resource and scopes pass.
/// Persistence and handlers must use this type rather than the untrusted input.
#[derive(Clone, Serialize)]
pub struct AuthorizationRequest {
    pub(crate) minimum_authentication: super::authentication_policy::AuthenticationPolicy,
    pub(crate) client_id: String,
    pub(crate) redirect_uri: String,
    pub(crate) scope: String,
    pub(crate) resource: String,
    pub(crate) audience: String,
    pub(crate) state: String,
    pub(crate) nonce: String,
    pub(crate) code_challenge: String,
    pub(crate) dpop_jkt: Option<String>,
    pub(crate) max_age: Option<u32>,
    pub(crate) prompt: Option<String>,
}

impl AuthorizationInput {
    pub fn validate(self, clients: &ClientRegistry) -> Result<AuthorizationRequest, OAuthError> {
        let client = clients.get(&self.client_id)?;
        // Compare original strings, never normalized or prefix-matched redirects.
        if !client.redirect_uris.contains(&self.redirect_uri) {
            return Err(OAuthError::InvalidClient);
        }
        if self.response_type != "code" {
            return Err(OAuthError::UnsupportedResponseType);
        }
        let allowed = client
            .resources
            .get(&self.resource)
            .ok_or(OAuthError::InvalidTarget)?;
        let resource_policy = allowed;
        let allowed = &resource_policy.scopes;
        let scopes = parse_scopes(&self.scope)?;
        if !scopes.contains("openid")
            || (resource_policy.audience == crate::tokens_policy::USERINFO_AUDIENCE
                && scopes
                    .iter()
                    .any(|s| *s != "offline_access" && !allowed.iter().any(|a| a == s)))
            || !scopes.iter().any(|s| allowed.iter().any(|a| a == s))
            || scopes.iter().any(|s| {
                !matches!(*s, "openid" | "profile" | "email")
                    && !(*s == "offline_access" && client.allow_refresh)
                    && !allowed.iter().any(|a| a == s)
            })
        {
            return Err(OAuthError::InvalidScope);
        }
        pkce::validate_challenge(&self.code_challenge, &self.code_challenge_method)?;
        if !bounded_transaction_value(&self.state)
            || !bounded_transaction_value(&self.nonce)
            || self.max_age.is_some_and(|v| v > 86_400)
            || self
                .prompt
                .as_deref()
                .is_some_and(|v| !matches!(v, "login" | "consent" | "none"))
            || (client.require_dpop && self.dpop_jkt.is_none())
            || self
                .dpop_jkt
                .as_deref()
                .is_some_and(|v| !pkce::is_sha256_base64url(v))
        {
            return Err(OAuthError::InvalidRequest);
        }
        Ok(AuthorizationRequest {
            minimum_authentication: client.minimum_authentication,
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            scope: scopes.into_iter().collect::<Vec<_>>().join(" "),
            resource: self.resource,
            audience: resource_policy.audience.clone(),
            state: self.state,
            nonce: self.nonce,
            code_challenge: self.code_challenge,
            dpop_jkt: self.dpop_jkt,
            max_age: self.max_age,
            prompt: self.prompt,
        })
    }
}

impl AuthorizationRequest {
    /// Restores persisted input through the current registration policy. Deleted
    /// clients, removed redirects and narrowed scopes invalidate pending work.
    pub(crate) fn restore(
        value: serde_json::Value,
        clients: &ClientRegistry,
    ) -> Result<Self, OAuthError> {
        let mut value = value;
        let object = value.as_object_mut().ok_or(OAuthError::InvalidRequest)?;
        let stored_audience = object
            .get("audience")
            .and_then(serde_json::Value::as_str)
            .ok_or(OAuthError::InvalidTarget)?
            .to_owned();
        object.insert("response_type".into(), serde_json::json!("code"));
        object.insert("code_challenge_method".into(), serde_json::json!("S256"));
        let input: AuthorizationInput =
            serde_json::from_value(value).map_err(|_| OAuthError::InvalidRequest)?;
        let restored = input.validate(clients)?;
        if restored.audience != stored_audience {
            return Err(OAuthError::InvalidTarget);
        }
        Ok(restored)
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
    pub fn resource(&self) -> &str {
        &self.resource
    }
    pub fn audience(&self) -> &str {
        &self.audience
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

fn parse_scopes(scope: &str) -> Result<BTreeSet<&str>, OAuthError> {
    if scope.is_empty() || scope.len() > 1024 {
        return Err(OAuthError::InvalidScope);
    }
    let mut scopes = BTreeSet::new();
    for value in scope.split(' ') {
        if value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
            || !scopes.insert(value)
        {
            return Err(OAuthError::InvalidScope);
        }
    }
    Ok(scopes)
}

fn bounded_transaction_value(value: &str) -> bool {
    (16..=512).contains(&value.len()) && value.bytes().all(|b| (0x21..=0x7e).contains(&b))
}
