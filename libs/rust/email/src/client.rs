use std::time::Duration;

use chrono::Utc;
use tonic::{
    Code, Request,
    metadata::{Ascii, MetadataValue},
    transport::{Channel, ClientTlsConfig, Endpoint},
};

use crate::{
    EmailCommand, EmailReceipt,
    command::{EmailCommandError, datetime},
    proto::nvbes::email::v1::{
        EmailReceipt as ProtoEmailReceipt,
        email_delivery_service_client::EmailDeliveryServiceClient,
    },
};

const ENDPOINT_ENV: &str = "NVBES_EMAIL_GRPC_ENDPOINT";
const AUTH_TOKEN_ENV: &str = "NVBES_EMAIL_GRPC_AUTH_TOKEN";
const MIN_AUTH_TOKEN_LENGTH: usize = 32;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, thiserror::Error)]
pub enum EmailClientError {
    #[error("email command is invalid")]
    InvalidCommand(#[source] EmailCommandError),
    #[error("email command conflicts with an existing idempotency key")]
    Conflict,
    #[error("email command service is unavailable")]
    Unavailable,
    #[error("email command service rejected the caller")]
    Unauthorized,
    #[error("email command service returned an invalid response")]
    Protocol,
    #[error("email client configuration is invalid: {0}")]
    Configuration(String),
}

#[derive(Clone)]
pub struct EmailClientConfig {
    pub(crate) endpoint: Endpoint,
    pub(crate) authorization: MetadataValue<Ascii>,
    pub(crate) call_timeout: Duration,
}

impl EmailClientConfig {
    pub fn from_env(environment: &str) -> Result<Self, EmailClientError> {
        let endpoint = std::env::var(ENDPOINT_ENV)
            .map_err(|_| EmailClientError::Configuration(format!("{ENDPOINT_ENV} is required")))?;
        let auth_token = std::env::var(AUTH_TOKEN_ENV).map_err(|_| {
            EmailClientError::Configuration(format!("{AUTH_TOKEN_ENV} is required"))
        })?;
        Self::from_values(environment, endpoint, auth_token, DEFAULT_TIMEOUT)
    }

    pub fn from_values(
        environment: &str,
        endpoint: String,
        auth_token: String,
        call_timeout: Duration,
    ) -> Result<Self, EmailClientError> {
        if auth_token.len() < MIN_AUTH_TOKEN_LENGTH {
            return Err(EmailClientError::Configuration(format!(
                "{AUTH_TOKEN_ENV} must contain at least {MIN_AUTH_TOKEN_LENGTH} characters"
            )));
        }
        if call_timeout.is_zero() {
            return Err(EmailClientError::Configuration(
                "email gRPC call timeout must be positive".to_string(),
            ));
        }
        let endpoint = Endpoint::from_shared(endpoint).map_err(|_| {
            EmailClientError::Configuration(format!("{ENDPOINT_ENV} is not a valid URI"))
        })?;
        let is_https = endpoint.uri().scheme_str() == Some("https");
        if !matches!(environment, "development" | "test") && !is_https {
            return Err(EmailClientError::Configuration(format!(
                "{ENDPOINT_ENV} must use https outside development"
            )));
        }
        let endpoint = if is_https {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            endpoint
                .tls_config(ClientTlsConfig::new().with_webpki_roots())
                .map_err(|_| {
                    EmailClientError::Configuration("email gRPC TLS is invalid".to_string())
                })?
        } else {
            endpoint
        }
        .connect_timeout(call_timeout)
        .timeout(call_timeout);
        let authorization = format!("Bearer {auth_token}").parse().map_err(|_| {
            EmailClientError::Configuration(format!("{AUTH_TOKEN_ENV} is not metadata-safe"))
        })?;

        Ok(Self {
            endpoint,
            authorization,
            call_timeout,
        })
    }
}

#[derive(Clone)]
pub struct EmailClient {
    inner: EmailDeliveryServiceClient<Channel>,
    authorization: MetadataValue<Ascii>,
    call_timeout: Duration,
}

impl EmailClient {
    pub async fn connect(config: EmailClientConfig) -> Result<Self, EmailClientError> {
        let channel = config
            .endpoint
            .connect()
            .await
            .map_err(|_| EmailClientError::Unavailable)?;
        Ok(Self {
            inner: EmailDeliveryServiceClient::new(channel),
            authorization: config.authorization,
            call_timeout: config.call_timeout,
        })
    }

    pub async fn send(&self, command: EmailCommand) -> Result<EmailReceipt, EmailClientError> {
        command
            .validate(Utc::now())
            .map_err(EmailClientError::InvalidCommand)?;
        let mut request = Request::new(command.into_proto());
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(self.call_timeout);

        let mut client = self.inner.clone();
        let response = client
            .submit_email(request)
            .await
            .map_err(map_status)?
            .into_inner();
        receipt(response)
    }
}

fn receipt(value: ProtoEmailReceipt) -> Result<EmailReceipt, EmailClientError> {
    let accepted_at = value
        .accepted_at
        .ok_or(EmailClientError::Protocol)
        .and_then(|value| datetime(value, "accepted_at").map_err(|_| EmailClientError::Protocol))?;
    let deliver_before = value
        .deliver_before
        .ok_or(EmailClientError::Protocol)
        .and_then(|value| {
            datetime(value, "deliver_before").map_err(|_| EmailClientError::Protocol)
        })?;
    if value.message_id.is_empty() || deliver_before <= accepted_at {
        return Err(EmailClientError::Protocol);
    }
    Ok(EmailReceipt {
        message_id: value.message_id,
        accepted_at,
        deliver_before,
        duplicate: value.duplicate,
    })
}

fn map_status(status: tonic::Status) -> EmailClientError {
    match status.code() {
        Code::InvalidArgument => {
            EmailClientError::InvalidCommand(EmailCommandError::field("remote_validation"))
        }
        Code::AlreadyExists => EmailClientError::Conflict,
        Code::Unauthenticated | Code::PermissionDenied => EmailClientError::Unauthorized,
        Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted => {
            EmailClientError::Unavailable
        }
        _ => EmailClientError::Protocol,
    }
}

#[cfg(test)]
#[path = "client.integration.tests.rs"]
mod integration_tests;
#[cfg(test)]
#[path = "client.tests.rs"]
mod tests;
