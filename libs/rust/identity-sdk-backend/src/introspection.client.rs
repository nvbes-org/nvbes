use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    Client, StatusCode, Url,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

/// Claims from a locally verified access token; never from an unverified decode.
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExpectedToken {
    pub sub: String,
    pub sid: String,
    pub grant_id: String,
    pub jti: String,
    pub iss: String,
    pub aud: String,
    pub client_id: String,
    pub scope: String,
    pub exp: u64,
    pub iat: u64,
    pub nbf: u64,
    pub token_type: String,
    pub cnf: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum IntrospectionError {
    #[error("invalid Identity introspection configuration")]
    Configuration,
    #[error("Identity activity verification is unavailable")]
    Unavailable,
}

#[derive(Clone)]
pub struct IntrospectionClient {
    http: Client,
    endpoint: Url,
    authorization: HeaderValue,
    capacity: Arc<Semaphore>,
}

impl IntrospectionClient {
    pub fn new(issuer: &str, client_id: &str, secret: &str) -> Result<Self, IntrospectionError> {
        let invalid = || IntrospectionError::Configuration;
        let url = Url::parse(issuer).map_err(|_| invalid())?;
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if url.host_str().is_none()
            || !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || client_id.is_empty()
            || client_id.len() > 64
            || !client_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(invalid());
        }
        let decoded = URL_SAFE_NO_PAD.decode(secret).map_err(|_| invalid())?;
        if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(&decoded) != secret {
            return Err(invalid());
        }
        let endpoint = Url::parse(&format!(
            "{}/oauth/introspect",
            issuer.trim_end_matches('/')
        ))
        .map_err(|_| invalid())?;
        let mut authorization = HeaderValue::from_str(&format!(
            "Basic {}",
            STANDARD.encode(format!("{client_id}:{secret}"))
        ))
        .map_err(|_| invalid())?;
        authorization.set_sensitive(true);
        let http = Client::builder()
            .timeout(Duration::from_millis(2500))
            .connect_timeout(Duration::from_secs(1))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .pool_max_idle_per_host(2)
            .build()
            .map_err(|_| invalid())?;
        Ok(Self {
            http,
            endpoint,
            authorization,
            capacity: Arc::new(Semaphore::new(16)),
        })
    }

    /// No positive cache or retry: callers must deny access on every error.
    pub async fn is_active(
        &self,
        token: &str,
        expected: &ExpectedToken,
    ) -> Result<bool, IntrospectionError> {
        let unavailable = || IntrospectionError::Unavailable;
        if token.is_empty() || token.len() > 16_384 {
            return Err(unavailable());
        }
        let _permit = self.capacity.try_acquire().map_err(|_| unavailable())?;
        let mut response = self
            .http
            .post(self.endpoint.clone())
            .header(AUTHORIZATION, self.authorization.clone())
            .form(&[("token", token), ("token_type_hint", "access_token")])
            .send()
            .await
            .map_err(|_| unavailable())?;
        if response.status() != StatusCode::OK
            || response.content_length().is_some_and(|n| n > 16_384)
            || response
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(';').next())
                .map(str::trim)
                != Some("application/json")
        {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len() + chunk.len() > 16_384 {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        #[derive(Deserialize)]
        struct Status {
            active: bool,
        }
        let status: Status = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        match status.active {
            false => Ok(false),
            true => {
                let actual: ExpectedToken =
                    serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
                if &actual != expected {
                    return Err(unavailable());
                }
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| unavailable())?
                    .as_secs();
                Ok(actual.exp > now)
            }
        }
    }
}

#[cfg(test)]
#[path = "introspection.client.tests.rs"]
mod tests;
