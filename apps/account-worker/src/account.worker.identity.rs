use std::{fmt, time::Duration};

use reqwest::StatusCode;

#[derive(Clone)]
pub struct IdentityClient {
    http: reqwest::Client,
    projection_endpoint: reqwest::Url,
    closure_endpoint: reqwest::Url,
    token: String,
}

#[derive(Clone)]
pub struct ClosureClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    token: String,
    operation: Operation,
}

#[derive(Debug)]
pub struct DispatchError {
    retryable: bool,
    code: &'static str,
}

impl IdentityClient {
    pub fn new(base_url: &str, token: String) -> anyhow::Result<Self> {
        let base_url = reqwest::Url::parse(base_url)?;
        let mut projection_endpoint = base_url.clone();
        projection_endpoint.set_path("/internal/v1/oidc-profile-projections");
        let mut closure_endpoint = base_url;
        closure_endpoint.set_path("/internal/v1/account-closures");
        let http = build_http_client()?;
        Ok(Self {
            http,
            projection_endpoint,
            closure_endpoint,
            token,
        })
    }

    pub async fn dispatch_profile(&self, payload: &serde_json::Value) -> Result<(), DispatchError> {
        self.dispatch(
            self.projection_endpoint.clone(),
            payload,
            Operation::ProfileProjection,
        )
        .await
    }

    pub async fn close_account(&self, payload: &serde_json::Value) -> Result<(), DispatchError> {
        self.dispatch(
            self.closure_endpoint.clone(),
            payload,
            Operation::AccountClosure,
        )
        .await
    }

    async fn dispatch(
        &self,
        endpoint: reqwest::Url,
        payload: &serde_json::Value,
        operation: Operation,
    ) -> Result<(), DispatchError> {
        dispatch(&self.http, endpoint, &self.token, payload, operation).await
    }
}

impl ClosureClient {
    pub fn new(base_url: &str, token: String, service: ClosureService) -> anyhow::Result<Self> {
        let mut endpoint = reqwest::Url::parse(base_url)?;
        endpoint.set_path("/internal/v1/account-closures");
        Ok(Self {
            http: build_http_client()?,
            endpoint,
            token,
            operation: match service {
                ClosureService::Cloud => Operation::CloudClosure,
                ClosureService::Billing => Operation::BillingClosure,
            },
        })
    }

    pub async fn close_account(&self, payload: &serde_json::Value) -> Result<(), DispatchError> {
        dispatch(
            &self.http,
            self.endpoint.clone(),
            &self.token,
            payload,
            self.operation,
        )
        .await
    }
}

#[derive(Clone, Copy)]
pub enum ClosureService {
    Cloud,
    Billing,
}

fn build_http_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .user_agent("nvbes-account-worker/1")
        .build()?)
}

async fn dispatch(
    http: &reqwest::Client,
    endpoint: reqwest::Url,
    token: &str,
    payload: &serde_json::Value,
    operation: Operation,
) -> Result<(), DispatchError> {
    let response = http
        .post(endpoint)
        .bearer_auth(token)
        .json(payload)
        .send()
        .await
        .map_err(|_| DispatchError::transient(operation.unreachable_code()))?;
    if response.status().is_success() {
        return Ok(());
    }
    Err(DispatchError::from_status(response.status(), operation))
}

#[derive(Clone, Copy)]
enum Operation {
    ProfileProjection,
    AccountClosure,
    CloudClosure,
    BillingClosure,
}

impl Operation {
    const fn unreachable_code(self) -> &'static str {
        match self {
            Self::ProfileProjection => "identity_projection_unreachable",
            Self::AccountClosure => "identity_closure_unreachable",
            Self::CloudClosure => "cloud_closure_unreachable",
            Self::BillingClosure => "billing_closure_unreachable",
        }
    }
}

impl DispatchError {
    fn transient(code: &'static str) -> Self {
        Self {
            retryable: true,
            code,
        }
    }

    pub fn storage() -> Self {
        Self::transient("account_avatar_delete_failed")
    }

    fn from_status(status: StatusCode, operation: Operation) -> Self {
        Self {
            retryable: status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error(),
            code: if status == StatusCode::UNAUTHORIZED {
                match operation {
                    Operation::ProfileProjection => "identity_projection_unauthorized",
                    Operation::AccountClosure => "identity_closure_unauthorized",
                    Operation::CloudClosure => "cloud_closure_unauthorized",
                    Operation::BillingClosure => "billing_closure_unauthorized",
                }
            } else if status == StatusCode::CONFLICT {
                match operation {
                    Operation::ProfileProjection => "identity_projection_conflict",
                    Operation::AccountClosure => "identity_closure_conflict",
                    Operation::CloudClosure => "cloud_closure_conflict",
                    Operation::BillingClosure => "billing_closure_conflict",
                }
            } else if status.is_client_error() {
                match operation {
                    Operation::ProfileProjection => "identity_projection_rejected",
                    Operation::AccountClosure => "identity_closure_rejected",
                    Operation::CloudClosure => "cloud_closure_rejected",
                    Operation::BillingClosure => "billing_closure_rejected",
                }
            } else {
                match operation {
                    Operation::ProfileProjection => "identity_projection_unavailable",
                    Operation::AccountClosure => "identity_closure_unavailable",
                    Operation::CloudClosure => "cloud_closure_unavailable",
                    Operation::BillingClosure => "billing_closure_unavailable",
                }
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

    use super::{DispatchError, Operation};

    #[test]
    fn retries_only_transient_http_failures() {
        assert!(
            DispatchError::from_status(StatusCode::TOO_MANY_REQUESTS, Operation::AccountClosure)
                .is_retryable()
        );
        assert!(
            DispatchError::from_status(StatusCode::BAD_GATEWAY, Operation::ProfileProjection)
                .is_retryable()
        );
        assert!(
            !DispatchError::from_status(StatusCode::BAD_REQUEST, Operation::AccountClosure)
                .is_retryable()
        );
        assert!(
            !DispatchError::from_status(StatusCode::UNAUTHORIZED, Operation::ProfileProjection)
                .is_retryable()
        );
    }
}
