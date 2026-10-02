use axum::http::HeaderMap;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde::Deserialize;
use subtle::ConstantTimeEq;

/// Resource credentials are separate from public OAuth clients and never logged.
pub struct ResourceServers(Vec<ResourceServer>);
struct ResourceServer {
    client_id: String,
    audience: String,
    secret: [u8; 32],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    client_id: String,
    audience: String,
    secret: String,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid Identity resource-server registry")]
pub struct RegistryError;

impl ResourceServers {
    pub fn from_json(json: &str) -> Result<Self, RegistryError> {
        if json.len() > 8192 {
            return Err(RegistryError);
        }
        let entries: Vec<Entry> = serde_json::from_str(json).map_err(|_| RegistryError)?;
        if entries.is_empty() || entries.len() > 8 {
            return Err(RegistryError);
        }
        let mut servers: Vec<ResourceServer> = Vec::new();
        for entry in entries {
            if entry.client_id.is_empty()
                || entry.client_id.len() > 64
                || !entry
                    .client_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                || !matches!(
                    entry.audience.as_str(),
                    "nvbes-account-service" | "nvbes-billing-service"
                )
            {
                return Err(RegistryError);
            }
            let secret = secret(&entry.secret).ok_or(RegistryError)?;
            if servers
                .iter()
                .any(|s| s.client_id == entry.client_id || bool::from(s.secret.ct_eq(&secret)))
            {
                return Err(RegistryError);
            }
            servers.push(ResourceServer {
                client_id: entry.client_id,
                audience: entry.audience,
                secret,
            });
        }
        Ok(Self(servers))
    }

    pub(crate) fn authenticate<'a>(&'a self, headers: &HeaderMap) -> Option<&'a str> {
        let mut values = headers.get_all("authorization").iter();
        let value = values.next()?.to_str().ok()?;
        if values.next().is_some() || value.len() > 256 {
            return None;
        }
        let decoded = STANDARD.decode(value.strip_prefix("Basic ")?).ok()?;
        let decoded = std::str::from_utf8(&decoded).ok()?;
        let (id, password) = decoded.split_once(':')?;
        let provided = secret(password)?;
        let server = self.0.iter().find(|s| s.client_id == id)?;
        bool::from(server.secret.ct_eq(&provided)).then_some(server.audience.as_str())
    }
}

fn secret(value: &str) -> Option<[u8; 32]> {
    if value.len() != 43 {
        return None;
    }
    let decoded: [u8; 32] = URL_SAFE_NO_PAD.decode(value).ok()?.try_into().ok()?;
    (URL_SAFE_NO_PAD.encode(decoded) == value).then_some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn registry_rejects_ambiguous_credentials_and_unapproved_resources() {
        let entry = json!({"client_id":"account-api","audience":"nvbes-account-service","secret":URL_SAFE_NO_PAD.encode([37;32])});
        assert!(ResourceServers::from_json(&json!([entry]).to_string()).is_ok());
        assert!(ResourceServers::from_json(&json!([entry, entry]).to_string()).is_err());
        for (key, value) in [
            ("audience", json!("nvbes-identity-userinfo")),
            ("client_id", json!("bad:id")),
            ("secret", json!("short")),
            ("unexpected", json!(true)),
        ] {
            let mut invalid = entry.clone();
            invalid[key] = value;
            assert!(ResourceServers::from_json(&json!([invalid]).to_string()).is_err());
        }
    }

    #[test]
    fn resource_authentication_rejects_duplicate_headers_and_foreign_secret() {
        let password = URL_SAFE_NO_PAD.encode([37; 32]);
        let registry = ResourceServers::from_json(&json!([{"client_id":"account-api","audience":"nvbes-account-service","secret":password}]).to_string()).unwrap();
        let mut headers = HeaderMap::new();
        let value = format!(
            "Basic {}",
            STANDARD.encode(format!("account-api:{password}"))
        );
        headers.insert("authorization", value.parse().unwrap());
        assert_eq!(
            registry.authenticate(&headers),
            Some("nvbes-account-service")
        );
        headers.append("authorization", value.parse().unwrap());
        assert!(registry.authenticate(&headers).is_none());
        for value in [
            "Bearer client-token".to_string(),
            "Basic invalid!".to_string(),
            format!(
                "Basic {}",
                STANDARD.encode(format!("account-api:{}", URL_SAFE_NO_PAD.encode([38; 32])))
            ),
        ] {
            headers.clear();
            headers.insert("authorization", value.parse().unwrap());
            assert!(registry.authenticate(&headers).is_none());
        }
    }
}
