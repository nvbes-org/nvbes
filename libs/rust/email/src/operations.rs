use std::time::Duration;

use serde_json::{Value, json};
use tonic::{
    Code, Request,
    metadata::{Ascii, MetadataValue},
    transport::Channel,
};

use crate::{
    EmailClientConfig,
    proto::nvbes::email::v1::{
        ApplyEmailSuppressionRequest, EmailOperationsSnapshot, EmailOperatorActionReceipt,
        EmailPrivacyActivity, GetEmailOperationsSnapshotRequest, GetEmailPrivacyActivityRequest,
        ReleaseEmailSuppressionRequest, ReplayEmailRequest, ReviewEmailSuppressionRequest,
        email_operations_service_client::EmailOperationsServiceClient,
    },
};

const DEVELOPMENT_ENDPOINT: &str = "http://127.0.0.1:3041";
const DEVELOPMENT_TOKEN: &str = "development-email-internal-token-32";
const OPERATIONS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum EmailOperationsError {
    #[error("email operations request is invalid")]
    Invalid,
    #[error("email operations target was not found")]
    NotFound,
    #[error("email operations precondition failed")]
    Conflict,
    #[error("email operations service rejected the caller")]
    Unauthorized,
    #[error("email operations service is unavailable")]
    Unavailable,
    #[error("email operations service returned an invalid response")]
    Protocol,
    #[error("email operations configuration is invalid: {0}")]
    Configuration(String),
}

#[derive(Clone)]
pub struct EmailOperationsClient {
    inner: EmailOperationsServiceClient<Channel>,
    authorization: MetadataValue<Ascii>,
    call_timeout: Duration,
}

impl EmailOperationsClient {
    pub fn from_environment(environment: &str) -> Result<Self, EmailOperationsError> {
        let config = if matches!(environment, "development" | "test") {
            EmailClientConfig::from_values(
                environment,
                std::env::var("NVBES_EMAIL_GRPC_ENDPOINT")
                    .unwrap_or_else(|_| DEVELOPMENT_ENDPOINT.to_string()),
                std::env::var("NVBES_EMAIL_GRPC_AUTH_TOKEN")
                    .unwrap_or_else(|_| DEVELOPMENT_TOKEN.to_string()),
                OPERATIONS_TIMEOUT,
            )
        } else {
            EmailClientConfig::from_env(environment)
        }
        .map_err(|error| EmailOperationsError::Configuration(error.to_string()))?;

        Ok(Self {
            inner: EmailOperationsServiceClient::new(config.endpoint.connect_lazy()),
            authorization: config.authorization,
            call_timeout: config.call_timeout,
        })
    }

    pub async fn snapshot(
        &self,
        value: GetEmailOperationsSnapshotRequest,
    ) -> Result<EmailOperationsSnapshot, EmailOperationsError> {
        self.inner
            .clone()
            .get_operations_snapshot(self.request(value))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn replay_email(
        &self,
        value: ReplayEmailRequest,
    ) -> Result<EmailOperatorActionReceipt, EmailOperationsError> {
        self.inner
            .clone()
            .replay_email(self.request(value))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn apply_suppression(
        &self,
        value: ApplyEmailSuppressionRequest,
    ) -> Result<EmailOperatorActionReceipt, EmailOperationsError> {
        self.inner
            .clone()
            .apply_suppression(self.request(value))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn release_suppression(
        &self,
        value: ReleaseEmailSuppressionRequest,
    ) -> Result<EmailOperatorActionReceipt, EmailOperationsError> {
        self.inner
            .clone()
            .release_suppression(self.request(value))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn review_suppression(
        &self,
        value: ReviewEmailSuppressionRequest,
    ) -> Result<EmailOperatorActionReceipt, EmailOperationsError> {
        self.inner
            .clone()
            .review_suppression(self.request(value))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    pub async fn privacy_activity(
        &self,
        value: GetEmailPrivacyActivityRequest,
    ) -> Result<EmailPrivacyActivity, EmailOperationsError> {
        self.inner
            .clone()
            .get_privacy_activity(self.request(value))
            .await
            .map(|response| response.into_inner())
            .map_err(map_status)
    }

    fn request<T>(&self, value: T) -> Request<T> {
        let mut request = Request::new(value);
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(self.call_timeout);
        request
    }
}

fn map_status(status: tonic::Status) -> EmailOperationsError {
    match status.code() {
        Code::InvalidArgument => EmailOperationsError::Invalid,
        Code::NotFound => EmailOperationsError::NotFound,
        Code::FailedPrecondition | Code::AlreadyExists => EmailOperationsError::Conflict,
        Code::Unauthenticated | Code::PermissionDenied => EmailOperationsError::Unauthorized,
        Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted => {
            EmailOperationsError::Unavailable
        }
        _ => EmailOperationsError::Protocol,
    }
}

pub fn privacy_activity_json(
    activity: EmailPrivacyActivity,
) -> Result<Value, EmailOperationsError> {
    let messages = activity
        .messages
        .into_iter()
        .map(|message| {
            Ok(json!({
                "id": message.id,
                "producer": message.producer,
                "business_type": message.business_type,
                "template_version": message.template_version,
                "status": message.status,
                "accepted_at": timestamp(message.accepted_at)?,
                "provider_accepted_at": optional_timestamp(message.provider_accepted_at)?,
                "terminal_at": optional_timestamp(message.terminal_at)?,
                "deliver_before": timestamp(message.deliver_before)?,
                "updated_at": timestamp(message.updated_at)?,
            }))
        })
        .collect::<Result<Vec<_>, EmailOperationsError>>()?;
    let provider_events = activity
        .provider_events
        .into_iter()
        .map(|event| {
            Ok(json!({
                "id": event.id,
                "provider": event.provider,
                "event_type": event.event_type,
                "received_at": timestamp(event.received_at)?,
                "processed_at": optional_timestamp(event.processed_at)?,
                "processing_result": event.processing_result,
            }))
        })
        .collect::<Result<Vec<_>, EmailOperationsError>>()?;
    Ok(json!({
        "messages": messages,
        "provider_events": provider_events,
    }))
}

fn timestamp(value: Option<prost_types::Timestamp>) -> Result<String, EmailOperationsError> {
    let value = value.ok_or(EmailOperationsError::Protocol)?;
    let nanos = u32::try_from(value.nanos).map_err(|_| EmailOperationsError::Protocol)?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(value.seconds, nanos)
        .map(|value| value.to_rfc3339())
        .ok_or(EmailOperationsError::Protocol)
}

fn optional_timestamp(
    value: Option<prost_types::Timestamp>,
) -> Result<Option<String>, EmailOperationsError> {
    value.map(|value| timestamp(Some(value))).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::nvbes::email::v1::{EmailPrivacyMessage, EmailPrivacyProviderEvent};

    fn at(seconds: i64) -> Option<prost_types::Timestamp> {
        Some(prost_types::Timestamp { seconds, nanos: 0 })
    }

    #[test]
    fn privacy_activity_projects_stable_json_without_recipient_secrets() {
        let value = privacy_activity_json(EmailPrivacyActivity {
            messages: vec![EmailPrivacyMessage {
                id: "message-1".to_string(),
                producer: "identity-service".to_string(),
                business_type: "account.security".to_string(),
                template_version: 1,
                status: "delivered".to_string(),
                accepted_at: at(1_700_000_000),
                provider_accepted_at: None,
                terminal_at: at(1_700_000_001),
                deliver_before: at(1_700_000_100),
                updated_at: at(1_700_000_001),
            }],
            provider_events: vec![EmailPrivacyProviderEvent {
                id: "event-1".to_string(),
                provider: "scaleway-tem".to_string(),
                event_type: "delivered".to_string(),
                received_at: at(1_700_000_001),
                processed_at: at(1_700_000_002),
                processing_result: Some("applied".to_string()),
            }],
        })
        .unwrap();

        assert_eq!(value["messages"][0]["id"], "message-1");
        assert_eq!(value["provider_events"][0]["event_type"], "delivered");
        assert!(!value.to_string().contains("recipient"));
    }
}
