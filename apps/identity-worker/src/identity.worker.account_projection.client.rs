use std::{fmt, time::Duration};

use reqwest::StatusCode;

#[derive(Clone)]
pub struct AccountProjectionClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    token: String,
}

#[derive(Debug)]
pub struct DispatchError {
    retryable: bool,
    code: &'static str,
}

impl AccountProjectionClient {
    pub fn from_env(environment: &str) -> anyhow::Result<Self> {
        let base_url = match std::env::var("NVBES_ACCOUNT_SERVICE_BASE_URL") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent)
                if matches!(environment, "development" | "test") =>
            {
                "http://localhost:4001".to_string()
            }
            Err(std::env::VarError::NotPresent) => {
                anyhow::bail!("NVBES_ACCOUNT_SERVICE_BASE_URL is required")
            }
            Err(error) => {
                anyhow::bail!("NVBES_ACCOUNT_SERVICE_BASE_URL could not be read: {error}")
            }
        };
        let mut endpoint = reqwest::Url::parse(base_url.trim())?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            anyhow::bail!("NVBES_ACCOUNT_SERVICE_BASE_URL must be an absolute HTTP(S) base URL");
        }
        endpoint.set_path("/internal/v1/identity-registrations");

        let token = match std::env::var("NVBES_ACCOUNT_PROVISIONING_TOKEN") {
            Ok(value) if value.trim().len() >= 32 => value.trim().to_string(),
            Ok(_) => anyhow::bail!(
                "NVBES_ACCOUNT_PROVISIONING_TOKEN must contain at least 32 characters"
            ),
            Err(std::env::VarError::NotPresent)
                if matches!(environment, "development" | "test") =>
            {
                "development-account-provisioning-token".to_string()
            }
            Err(std::env::VarError::NotPresent) => {
                anyhow::bail!("NVBES_ACCOUNT_PROVISIONING_TOKEN is required")
            }
            Err(error) => {
                anyhow::bail!("NVBES_ACCOUNT_PROVISIONING_TOKEN could not be read: {error}")
            }
        };
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .user_agent("nvbes-identity-worker/1")
            .build()?;
        Ok(Self {
            http,
            endpoint,
            token,
        })
    }

    pub async fn dispatch(&self, payload: &serde_json::Value) -> Result<(), DispatchError> {
        let response = self
            .http
            .post(self.endpoint.clone())
            .bearer_auth(&self.token)
            .json(payload)
            .send()
            .await
            .map_err(|_| DispatchError::transient("account_projection_unreachable"))?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(DispatchError::from_status(response.status()))
    }
}

impl DispatchError {
    fn transient(code: &'static str) -> Self {
        Self {
            retryable: true,
            code,
        }
    }

    fn from_status(status: StatusCode) -> Self {
        Self {
            retryable: status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error(),
            code: if status == StatusCode::UNAUTHORIZED {
                "account_projection_unauthorized"
            } else if status == StatusCode::CONFLICT {
                "account_projection_conflict"
            } else if status.is_client_error() {
                "account_projection_rejected"
            } else {
                "account_projection_unavailable"
            },
        }
    }

    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for DispatchError {}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;

    use super::DispatchError;

    #[test]
    fn only_transport_rate_limit_and_server_failures_are_retried() {
        assert!(DispatchError::from_status(StatusCode::TOO_MANY_REQUESTS).is_retryable());
        assert!(DispatchError::from_status(StatusCode::BAD_GATEWAY).is_retryable());
        assert!(!DispatchError::from_status(StatusCode::UNAUTHORIZED).is_retryable());
        assert!(!DispatchError::from_status(StatusCode::CONFLICT).is_retryable());
    }
}
