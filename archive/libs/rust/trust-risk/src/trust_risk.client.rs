use std::time::Duration;

use tonic::{
    Code, Request,
    metadata::{Ascii, MetadataValue},
    transport::{Channel, ClientTlsConfig, Endpoint},
};

use crate::proto::nvbes::trust_risk::v1::{
    AssessRiskRequest, RiskEvaluation, SubmitLabelsRequest, SubmitLabelsResponse,
    SubmitSignalsRequest, SubmitSignalsResponse,
    trust_risk_assessment_service_client::TrustRiskAssessmentServiceClient,
    trust_risk_label_service_client::TrustRiskLabelServiceClient,
    trust_risk_signal_service_client::TrustRiskSignalServiceClient,
};

const ENDPOINT_ENV: &str = "NVBES_TRUST_RISK_GRPC_ENDPOINT";
const AUTH_TOKEN_ENV: &str = "NVBES_TRUST_RISK_GRPC_AUTH_TOKEN";
const MIN_TOKEN_LENGTH: usize = 32;
const DEFAULT_ASSESSMENT_TIMEOUT: Duration = Duration::from_millis(100);
const DURABLE_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_MESSAGE_SIZE: usize = 256 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum TrustRiskClientError {
    #[error("trust/risk request is invalid")]
    Invalid,
    #[error("trust/risk request conflicts with an existing idempotency key")]
    Conflict,
    #[error("trust/risk service rejected the caller")]
    Unauthorized,
    #[error("trust/risk service is unavailable")]
    Unavailable,
    #[error("trust/risk service returned an invalid response")]
    Protocol,
    #[error("trust/risk client configuration is invalid: {0}")]
    Configuration(String),
}

#[derive(Debug, Clone)]
pub struct TrustRiskClientConfig {
    pub(crate) endpoint: Endpoint,
    pub(crate) authorization: MetadataValue<Ascii>,
    pub(crate) assessment_timeout: Duration,
    pub(crate) durable_timeout: Duration,
}

impl TrustRiskClientConfig {
    pub fn from_env(environment: &str) -> Result<Self, TrustRiskClientError> {
        let endpoint = std::env::var(ENDPOINT_ENV).map_err(|_| {
            TrustRiskClientError::Configuration(format!("{ENDPOINT_ENV} is required"))
        })?;
        let token = std::env::var(AUTH_TOKEN_ENV).map_err(|_| {
            TrustRiskClientError::Configuration(format!("{AUTH_TOKEN_ENV} is required"))
        })?;
        Self::from_values(environment, endpoint, token, DEFAULT_ASSESSMENT_TIMEOUT)
    }

    pub fn from_values(
        environment: &str,
        endpoint: String,
        token: String,
        assessment_timeout: Duration,
    ) -> Result<Self, TrustRiskClientError> {
        if token.len() < MIN_TOKEN_LENGTH {
            return Err(configuration("authentication token is too short"));
        }
        if assessment_timeout.is_zero() {
            return Err(configuration("assessment timeout must be positive"));
        }
        let endpoint = Endpoint::from_shared(endpoint)
            .map_err(|_| configuration("endpoint is not a valid URI"))?;
        let is_https = endpoint.uri().scheme_str() == Some("https");
        if !matches!(environment, "development" | "test") && !is_https {
            return Err(configuration("endpoint must use https outside development"));
        }
        let endpoint = if is_https {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            endpoint
                .tls_config(ClientTlsConfig::new().with_webpki_roots())
                .map_err(|_| configuration("TLS configuration is invalid"))?
        } else {
            endpoint
        }
        .connect_timeout(DURABLE_TIMEOUT);
        let authorization = format!("Bearer {token}")
            .parse()
            .map_err(|_| configuration("authentication token is not metadata-safe"))?;

        Ok(Self {
            endpoint,
            authorization,
            assessment_timeout,
            durable_timeout: DURABLE_TIMEOUT,
        })
    }
}

#[derive(Clone)]
pub struct TrustRiskClient {
    signals: TrustRiskSignalServiceClient<Channel>,
    assessments: TrustRiskAssessmentServiceClient<Channel>,
    labels: TrustRiskLabelServiceClient<Channel>,
    authorization: MetadataValue<Ascii>,
    assessment_timeout: Duration,
    durable_timeout: Duration,
}

impl TrustRiskClient {
    pub async fn connect(config: TrustRiskClientConfig) -> Result<Self, TrustRiskClientError> {
        let channel = config
            .endpoint
            .connect()
            .await
            .map_err(|_| TrustRiskClientError::Unavailable)?;
        Ok(Self::from_channel(channel, config))
    }

    pub fn connect_lazy(config: TrustRiskClientConfig) -> Self {
        let channel = config.endpoint.clone().connect_lazy();
        Self::from_channel(channel, config)
    }

    pub async fn submit_signals(
        &self,
        value: SubmitSignalsRequest,
    ) -> Result<SubmitSignalsResponse, TrustRiskClientError> {
        self.signals
            .clone()
            .submit_signals(self.request(value, self.durable_timeout))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn assess_risk(
        &self,
        value: AssessRiskRequest,
    ) -> Result<RiskEvaluation, TrustRiskClientError> {
        self.assessments
            .clone()
            .assess_risk(self.request(value, self.assessment_timeout))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn submit_labels(
        &self,
        value: SubmitLabelsRequest,
    ) -> Result<SubmitLabelsResponse, TrustRiskClientError> {
        self.labels
            .clone()
            .submit_labels(self.request(value, self.durable_timeout))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    fn from_channel(channel: Channel, config: TrustRiskClientConfig) -> Self {
        Self {
            signals: TrustRiskSignalServiceClient::new(channel.clone())
                .max_encoding_message_size(MAX_MESSAGE_SIZE)
                .max_decoding_message_size(MAX_MESSAGE_SIZE),
            assessments: TrustRiskAssessmentServiceClient::new(channel.clone())
                .max_encoding_message_size(MAX_MESSAGE_SIZE)
                .max_decoding_message_size(MAX_MESSAGE_SIZE),
            labels: TrustRiskLabelServiceClient::new(channel)
                .max_encoding_message_size(MAX_MESSAGE_SIZE)
                .max_decoding_message_size(MAX_MESSAGE_SIZE),
            authorization: config.authorization,
            assessment_timeout: config.assessment_timeout,
            durable_timeout: config.durable_timeout,
        }
    }

    fn request<T>(&self, value: T, timeout: Duration) -> Request<T> {
        let mut request = Request::new(value);
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(timeout);
        request
    }
}

fn configuration(message: &str) -> TrustRiskClientError {
    TrustRiskClientError::Configuration(message.to_string())
}

pub(crate) fn map_status(status: tonic::Status) -> TrustRiskClientError {
    match status.code() {
        Code::InvalidArgument => TrustRiskClientError::Invalid,
        Code::AlreadyExists | Code::FailedPrecondition => TrustRiskClientError::Conflict,
        Code::Unauthenticated | Code::PermissionDenied => TrustRiskClientError::Unauthorized,
        Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted => {
            TrustRiskClientError::Unavailable
        }
        _ => TrustRiskClientError::Protocol,
    }
}

#[cfg(test)]
#[path = "trust_risk.client.tests.rs"]
mod tests;
