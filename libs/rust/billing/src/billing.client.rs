use std::time::Duration;

use tonic::{
    metadata::{Ascii, MetadataValue},
    transport::{Channel, ClientTlsConfig, Endpoint},
};

use crate::proto::nvbes::billing::v1::{
    GetWorkspaceEntitlementsRequest, WorkspaceEntitlementsResponse,
    billing_delivery_service_client::BillingDeliveryServiceClient,
    billing_operations_service_client::BillingOperationsServiceClient,
};

const ENDPOINT_ENV: &str = "NVBES_BILLING_GRPC_ENDPOINT";
const AUTH_TOKEN_ENV: &str = "NVBES_BILLING_GRPC_AUTH_TOKEN";
const MIN_AUTH_TOKEN_LENGTH: usize = 32;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum BillingClientError {
    #[error("billing service is unavailable")]
    Unavailable,
    #[error("billing service rejected the caller")]
    Unauthorized,
    #[error("billing service returned an invalid response")]
    Protocol,
    #[error("billing client configuration is invalid: {0}")]
    Configuration(String),
}

#[derive(Clone)]
pub struct BillingClientConfig {
    pub(crate) endpoint: Endpoint,
    pub(crate) authorization: MetadataValue<Ascii>,
    pub(crate) call_timeout: Duration,
}

impl BillingClientConfig {
    pub fn from_env(environment: &str) -> Result<Self, BillingClientError> {
        let endpoint = std::env::var(ENDPOINT_ENV).map_err(|_| {
            BillingClientError::Configuration(format!("{ENDPOINT_ENV} is required"))
        })?;
        let auth_token = std::env::var(AUTH_TOKEN_ENV).map_err(|_| {
            BillingClientError::Configuration(format!("{AUTH_TOKEN_ENV} is required"))
        })?;
        Self::from_values(environment, endpoint, auth_token, DEFAULT_TIMEOUT)
    }

    pub fn from_values(
        environment: &str,
        endpoint: String,
        auth_token: String,
        call_timeout: Duration,
    ) -> Result<Self, BillingClientError> {
        if auth_token.len() < MIN_AUTH_TOKEN_LENGTH {
            return Err(BillingClientError::Configuration(format!(
                "{AUTH_TOKEN_ENV} must contain at least {MIN_AUTH_TOKEN_LENGTH} characters"
            )));
        }
        if call_timeout.is_zero() {
            return Err(BillingClientError::Configuration(
                "billing gRPC call timeout must be positive".to_string(),
            ));
        }
        let endpoint = Endpoint::from_shared(endpoint).map_err(|_| {
            BillingClientError::Configuration(format!("{ENDPOINT_ENV} is not a valid URI"))
        })?;
        let is_https = endpoint.uri().scheme_str() == Some("https");
        if !matches!(environment, "development" | "test") && !is_https {
            return Err(BillingClientError::Configuration(format!(
                "{ENDPOINT_ENV} must use https outside development"
            )));
        }
        let endpoint = if is_https {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            endpoint
                .tls_config(ClientTlsConfig::new().with_webpki_roots())
                .map_err(|_| {
                    BillingClientError::Configuration("billing gRPC TLS is invalid".to_string())
                })?
        } else {
            endpoint
        }
        .connect_timeout(call_timeout)
        .timeout(call_timeout);

        let authorization = format!("Bearer {auth_token}").parse().map_err(|_| {
            BillingClientError::Configuration(format!("{AUTH_TOKEN_ENV} is not metadata-safe"))
        })?;

        Ok(Self {
            endpoint,
            authorization,
            call_timeout,
        })
    }
}

/// Typed gRPC client for internal callers of BillingDeliveryService.
#[derive(Clone)]
pub struct BillingClient {
    delivery: BillingDeliveryServiceClient<Channel>,
    _operations: BillingOperationsServiceClient<Channel>,
    authorization: MetadataValue<Ascii>,
    call_timeout: Duration,
}

impl BillingClient {
    pub async fn connect(config: BillingClientConfig) -> Result<Self, BillingClientError> {
        let channel = config
            .endpoint
            .connect()
            .await
            .map_err(|_| BillingClientError::Unavailable)?;
        Ok(Self {
            delivery: BillingDeliveryServiceClient::new(channel.clone()),
            _operations: BillingOperationsServiceClient::new(channel),
            authorization: config.authorization,
            call_timeout: config.call_timeout,
        })
    }

    pub async fn get_workspace_entitlements(
        &self,
        workspace_id: String,
    ) -> Result<WorkspaceEntitlementsResponse, BillingClientError> {
        let mut req = tonic::Request::new(GetWorkspaceEntitlementsRequest {
            context: None,
            workspace_id,
        });
        req.metadata_mut()
            .insert("authorization", self.authorization.clone());
        req.set_timeout(self.call_timeout);

        let mut client = self.delivery.clone();
        client
            .get_workspace_entitlements(req)
            .await
            .map(|r| r.into_inner())
            .map_err(map_status)
    }
}

fn map_status(status: tonic::Status) -> BillingClientError {
    match status.code() {
        tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => {
            BillingClientError::Unauthorized
        }
        tonic::Code::Unavailable
        | tonic::Code::DeadlineExceeded
        | tonic::Code::ResourceExhausted => BillingClientError::Unavailable,
        _ => BillingClientError::Protocol,
    }
}
