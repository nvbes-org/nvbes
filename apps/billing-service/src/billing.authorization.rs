use crate::error::BillingError;
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Principal,
    #[default]
    Team,
}
impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Principal => "principal",
            Self::Team => "team",
        }
    }
}

#[derive(Deserialize)]
pub struct BillingAccount {
    pub id: Uuid,
    // The existing /workspaces/{id} routes are explicitly team-only aliases.
    #[serde(default)]
    pub account_type: AccountType,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Decision {
    principal_id: Uuid,
    account_id: Uuid,
    account_type: AccountType,
    allowed: bool,
}

#[derive(Clone)]
pub struct AccountAuthority {
    client: Client,
    endpoint: Url,
    permits: Arc<Semaphore>,
}

impl AccountAuthority {
    pub fn new(origin: &str, secret: &str) -> anyhow::Result<Self> {
        let mut endpoint = Url::parse(origin)?;
        anyhow::ensure!(
            (endpoint.scheme() == "https"
                || endpoint.scheme() == "http"
                    && matches!(
                        endpoint.host_str(),
                        Some("localhost" | "127.0.0.1" | "[::1]")
                    ))
                && endpoint.host_str().is_some()
                && endpoint.username().is_empty()
                && endpoint.password().is_none()
                && endpoint.query().is_none()
                && endpoint.fragment().is_none()
                && endpoint.path() == "/",
            "invalid Account authorization origin"
        );
        anyhow::ensure!(
            secret.len() == 64
                && secret
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid Account authorization credential"
        );
        endpoint.set_path("/internal/v1/billing/authorize");
        let mut credential = HeaderValue::from_str(&format!("Bearer {secret}"))?;
        credential.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, credential);
        let client = Client::builder()
            .default_headers(headers)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_millis(2500))
            .pool_max_idle_per_host(2)
            .build()?;
        Ok(Self {
            client,
            endpoint,
            permits: Arc::new(Semaphore::new(16)),
        })
    }

    pub async fn require(
        &self,
        principal_id: Uuid,
        account: &BillingAccount,
    ) -> Result<(), BillingError> {
        if principal_id.is_nil() || account.id.is_nil() {
            return Err(BillingError::AccountForbidden);
        }
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| BillingError::AccountUnavailable)?;
        let expected = Decision {
            principal_id,
            account_id: account.id,
            account_type: account.account_type,
            allowed: true,
        };
        let mut response = self.client.post(self.endpoint.clone())
            .json(&serde_json::json!({"principal_id":principal_id,"account_id":account.id,"account_type":account.account_type}))
            .send().await.map_err(|_| BillingError::AccountUnavailable)?;
        if response.status() != reqwest::StatusCode::OK
            || !response
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .is_some_and(|h| {
                    h.split(';')
                        .next()
                        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
                })
            || response.content_length().is_some_and(|n| n > 1024)
        {
            return Err(BillingError::AccountUnavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| BillingError::AccountUnavailable)?
        {
            if bytes.len() + chunk.len() > 1024 {
                return Err(BillingError::AccountUnavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        let mut decision: Decision =
            serde_json::from_slice(&bytes).map_err(|_| BillingError::AccountUnavailable)?;
        let allowed = decision.allowed;
        decision.allowed = true;
        if decision != expected {
            return Err(BillingError::AccountUnavailable);
        }
        if !allowed {
            return Err(BillingError::AccountForbidden);
        }
        Ok(())
    }
}

pub async fn require(
    authority: Option<&AccountAuthority>,
    principal: Uuid,
    account: &BillingAccount,
) -> Result<(), BillingError> {
    authority
        .ok_or(BillingError::AccountUnavailable)?
        .require(principal, account)
        .await
}

#[cfg(test)]
#[path = "billing.authorization.tests.rs"]
mod tests;
