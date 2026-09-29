use crate::oauth::{error::OAuthError, pkce::is_sha256_base64url, request::AuthorizationInput};
use std::collections::BTreeMap;

pub(super) enum AuthorizationQuery {
    Direct(AuthorizationInput),
    Par { client_id: String, handle: String },
}

impl AuthorizationQuery {
    pub(super) fn decode(raw: &str) -> Result<Self, OAuthError> {
        if raw.is_empty() || raw.len() > 8192 {
            return Err(OAuthError::InvalidRequest);
        }
        let mut url = reqwest::Url::parse("https://identity.invalid/").expect("static URL");
        url.set_query(Some(raw));
        Self::from_fields(url.query_pairs().into_owned().collect())
    }

    pub(super) fn from_fields(pairs: Vec<(String, String)>) -> Result<Self, OAuthError> {
        let mut fields = BTreeMap::new();
        for (key, value) in pairs {
            if fields.len() >= 32 || fields.insert(key, value).is_some() {
                return Err(OAuthError::InvalidRequest);
            }
        }
        // JAR is not implemented. Never silently fall back from a signed request.
        if fields.contains_key("request") {
            return Err(OAuthError::InvalidRequest);
        }
        if let Some(uri) = fields.get("request_uri") {
            if fields.len() != 2 {
                return Err(OAuthError::InvalidRequest);
            }
            let client_id = fields
                .get("client_id")
                .filter(|id| !id.is_empty())
                .ok_or(OAuthError::InvalidRequest)?
                .clone();
            let handle = uri
                .strip_prefix("urn:ietf:params:oauth:request_uri:")
                .filter(|handle| is_sha256_base64url(handle))
                .ok_or(OAuthError::InvalidRequest)?
                .to_owned();
            return Ok(Self::Par { client_id, handle });
        }
        let max_age = fields
            .remove("max_age")
            .map(|value| {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(OAuthError::InvalidRequest);
                }
                value.parse::<u32>().map_err(|_| OAuthError::InvalidRequest)
            })
            .transpose()?;
        let mut value = serde_json::to_value(fields).map_err(|_| OAuthError::InvalidRequest)?;
        if let Some(max_age) = max_age {
            value["max_age"] = max_age.into();
        }
        let input = serde_json::from_value(value).map_err(|_| OAuthError::InvalidRequest)?;
        Ok(Self::Direct(input))
    }
}
