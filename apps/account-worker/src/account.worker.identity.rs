use std::{fmt, time::Duration};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::types::IdentityClosureRequest;

const IDENTITY_AUDIENCE: &str = "nvbes-identity-service";
const CLOSURE_SCOPE: &str = "identity:account:close";

#[derive(Clone)]
pub struct IdentityClosureClient {
    http: reqwest::Client,
    token_endpoint: String,
    closure_endpoint: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug)]
pub struct DispatchError {
    retryable: bool,
    code: &'static str,
    summary: String,
}

impl IdentityClosureClient {
    pub fn new(
        identity_service_base_url: &str,
        client_id: String,
        client_secret: String,
    ) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .user_agent("nvbes-account-worker/1")
            .build()?;
        Ok(Self {
            http,
            token_endpoint: format!(
                "{}/oauth/token",
                identity_service_base_url.trim_end_matches('/')
            ),
            closure_endpoint: format!(
                "{}/api/v1/internal/account-closures",
                identity_service_base_url.trim_end_matches('/')
            ),
            client_id,
            client_secret,
        })
    }

    pub async fn close_identity(
        &self,
        request: &IdentityClosureRequest,
    ) -> Result<(), DispatchError> {
        let access_token = self.request_access_token().await?;
        let response = self
            .http
            .post(&self.closure_endpoint)
            .bearer_auth(access_token)
            .header("Idempotency-Key", request.event_id.to_string())
            .json(request)
            .send()
            .await
            .map_err(|_| DispatchError::transient("identity_closure_unreachable"))?;

        if response.status().is_success() {
            return Ok(());
        }
        Err(status_error("identity_closure_rejected", response.status()))
    }

    async fn request_access_token(&self) -> Result<String, DispatchError> {
        let response = self
            .http
            .post(&self.token_endpoint)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&TokenRequest {
                grant_type: "client_credentials",
                audience: IDENTITY_AUDIENCE,
                scope: CLOSURE_SCOPE,
            })
            .send()
            .await
            .map_err(|_| DispatchError::transient("identity_token_unreachable"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(status_error("identity_token_rejected", status));
        }
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|_| DispatchError::permanent("identity_token_invalid_response"))?;
        if token.access_token.trim().is_empty() || !token.token_type.eq_ignore_ascii_case("bearer") {
            return Err(DispatchError::permanent(
                "identity_token_invalid_response",
            ));
        }
        Ok(token.access_token)
    }
}

impl DispatchError {
    fn transient(code: &'static str) -> Self {
        Self {
            retryable: true,
            code,
            summary: code.replace('_', " "),
        }
    }

    fn permanent(code: &'static str) -> Self {
        Self {
            retryable: false,
            code,
            summary: code.replace('_', " "),
        }
    }

    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub fn safe_message(&self) -> String {
        format!("{}: {}", self.code, self.summary)
    }
}

impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.safe_message())
    }
}

impl std::error::Error for DispatchError {}

fn status_error(code: &'static str, status: StatusCode) -> DispatchError {
    DispatchError {
        retryable: status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error(),
        code,
        summary: format!("Identity returned HTTP {}", status.as_u16()),
    }
}

#[derive(Serialize)]
struct TokenRequest<'a> {
    grant_type: &'a str,
    audience: &'a str,
    scope: &'a str,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;

    use super::status_error;

    #[test]
    fn retries_rate_limits_and_server_failures_only() {
        assert!(status_error("test", StatusCode::TOO_MANY_REQUESTS).is_retryable());
        assert!(status_error("test", StatusCode::BAD_GATEWAY).is_retryable());
        assert!(!status_error("test", StatusCode::BAD_REQUEST).is_retryable());
        assert!(!status_error("test", StatusCode::FORBIDDEN).is_retryable());
        assert!(!status_error("test", StatusCode::CONFLICT).is_retryable());
    }

    #[test]
    fn safe_error_never_contains_remote_response_body() {
        let error = status_error("identity_closure_rejected", StatusCode::FORBIDDEN);
        assert_eq!(
            error.safe_message(),
            "identity_closure_rejected: Identity returned HTTP 403"
        );
    }
}
