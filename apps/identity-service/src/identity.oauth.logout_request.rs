use super::{clients::ClientRegistry, error::OAuthError};
use crate::tokens::{LogoutHint, TokenService};
use std::collections::BTreeMap;

/// Validated request policy, not permission to revoke a session. HTTP handlers
/// must bind the browser/session and obtain confirmation before using the return.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogoutRequest {
    pub client_id: Option<String>,
    pub hint: Option<LogoutHint>,
    redirect_uri: Option<String>,
    state: Option<String>,
}

impl LogoutRequest {
    pub fn revalidate(&self, clients: &ClientRegistry) -> Result<(), OAuthError> {
        let client = self
            .client_id
            .as_deref()
            .map(|id| clients.get(id))
            .transpose()?;
        if self
            .hint
            .as_ref()
            .is_some_and(|hint| self.client_id.as_deref() != Some(&hint.client_id))
            || self.redirect_uri.as_ref().is_some_and(|uri| {
                self.hint.is_none()
                    || !client.is_some_and(|client| client.post_logout_redirect_uris.contains(uri))
            })
        {
            return Err(OAuthError::InvalidRequest);
        }
        Ok(())
    }
    /// GET query and POST form handlers must both use this duplicate-aware path.
    pub fn from_fields(
        fields: Vec<(String, String)>,
        clients: &ClientRegistry,
        tokens: &TokenService,
    ) -> Result<Self, OAuthError> {
        if fields.len() > 16
            || fields
                .iter()
                .map(|(k, v)| k.len().saturating_add(v.len()))
                .sum::<usize>()
                > 32_768
        {
            return Err(OAuthError::InvalidRequest);
        }
        let mut values = BTreeMap::new();
        for (name, value) in fields {
            if !matches!(
                name.as_str(),
                "id_token_hint"
                    | "client_id"
                    | "post_logout_redirect_uri"
                    | "state"
                    | "ui_locales"
                    | "logout_hint"
            ) || values.insert(name, value).is_some()
            {
                return Err(OAuthError::InvalidRequest);
            }
        }
        let hint = values
            .get("id_token_hint")
            .map(|value| {
                tokens
                    .verify_logout_hint(value)
                    .map_err(|_| OAuthError::InvalidRequest)
            })
            .transpose()?;
        let explicit_client = values.remove("client_id");
        if let (Some(client), Some(hint)) = (&explicit_client, &hint) {
            if client != &hint.client_id {
                return Err(OAuthError::InvalidClient);
            }
        }
        let client_id =
            explicit_client.or_else(|| hint.as_ref().map(|hint| hint.client_id.clone()));
        let client = client_id.as_deref().map(|id| clients.get(id)).transpose()?;
        let redirect_uri = values.remove("post_logout_redirect_uri");
        if let Some(uri) = &redirect_uri {
            // Registration alone does not prove the RP session. This initial
            // profile requires its signed ID Token before permitting a return.
            if hint.is_none()
                || !client.is_some_and(|client| client.post_logout_redirect_uris.contains(uri))
            {
                return Err(OAuthError::InvalidRequest);
            }
        }
        let state = values.remove("state");
        if state
            .as_ref()
            .is_some_and(|s| s.len() > 1024 || s.chars().any(char::is_control))
        {
            return Err(OAuthError::InvalidRequest);
        }
        Ok(Self {
            client_id,
            hint,
            redirect_uri,
            state,
        })
    }

    /// Only return this target after confirmed logout. State is encoded as a
    /// query value and cannot replace the registered origin, path or parameters.
    pub fn return_uri(&self) -> Result<Option<String>, OAuthError> {
        self.redirect_uri
            .as_ref()
            .map(|uri| {
                let mut target =
                    reqwest::Url::parse(uri).map_err(|_| OAuthError::InvalidRequest)?;
                if let Some(state) = &self.state {
                    target.query_pairs_mut().append_pair("state", state);
                }
                Ok(target.to_string())
            })
            .transpose()
    }
}

#[cfg(test)]
#[path = "identity.oauth.logout_request.tests.rs"]
mod tests;
