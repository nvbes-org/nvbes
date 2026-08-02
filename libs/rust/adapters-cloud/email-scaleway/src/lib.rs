use async_trait::async_trait;
use chrono::{DateTime, Utc};
use nvbes_email::{EmailError, EmailMessage, EmailSender, SendResult};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Clone)]
pub struct ScalewayEmailClient {
    client: reqwest::Client,
    api_url: String,
    secret_key: String,
    project_id: String,
}

#[derive(Debug, Deserialize)]
struct ScalewayEmailResponse {
    emails: Vec<ScalewayEmail>,
}

#[derive(Debug, Deserialize)]
struct ScalewayEmail {
    id: String,
}

#[derive(Debug, Serialize)]
struct ScalewayAddress {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScalewayHeader {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct ScalewayEmailRequest {
    from: ScalewayAddress,
    to: Vec<ScalewayAddress>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_before: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    additional_headers: Vec<ScalewayHeader>,
}

impl ScalewayEmailClient {
    pub fn new(secret_key: String, project_id: String, region: &str) -> Self {
        let api_url = format!(
            "https://api.scaleway.com/transactional-email/v1alpha1/regions/{region}/emails"
        );

        Self {
            client: reqwest::Client::builder()
                .min_tls_version(reqwest::tls::Version::TLS_1_3)
                .https_only(true)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Scaleway email client should initialize"),
            api_url,
            secret_key,
            project_id,
        }
    }

    async fn send(
        &self,
        message: &EmailMessage,
        send_before: Option<DateTime<Utc>>,
    ) -> Result<SendResult, EmailError> {
        let request = ScalewayEmailRequest {
            from: ScalewayAddress {
                email: message.from.email.clone(),
                name: message.from.name.clone(),
            },
            to: message
                .to
                .iter()
                .map(|addr| ScalewayAddress {
                    email: addr.email.clone(),
                    name: addr.name.clone(),
                })
                .collect(),
            subject: message.subject.clone(),
            text: message.text_body.clone(),
            html: message.html_body.clone(),
            project_id: self.project_id.clone(),
            send_before,
            additional_headers: message
                .headers
                .iter()
                .map(|(key, value)| ScalewayHeader {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect(),
        };

        debug!(
            recipient_count = request.to.len(),
            "sending transactional email through Scaleway"
        );
        let response = nvbes_core::trace_context::with_fresh_trace_headers(
            self.client
                .post(&self.api_url)
                .header("X-Auth-Token", &self.secret_key)
                .json(&request),
        )
        .send()
        .await?;
        let status = response.status();

        if !status.is_success() {
            let body_size_bytes = response.bytes().await.map(|body| body.len()).unwrap_or(0);
            error!(%status, body_size_bytes, "Scaleway transactional email API rejected request");
            return Err(EmailError::Api {
                status: status.as_u16(),
                message: "redacted provider response".to_string(),
            });
        }

        let response: ScalewayEmailResponse = response.json().await?;
        let provider_email_id = response
            .emails
            .into_iter()
            .next()
            .map(|email| email.id)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| EmailError::Api {
                status: status.as_u16(),
                message: "provider response contained no email".to_string(),
            })?;
        debug!(provider_email_id, "Scaleway accepted transactional email");
        Ok(SendResult { provider_email_id })
    }
}

#[async_trait]
impl EmailSender for ScalewayEmailClient {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError> {
        self.send(message, None).await
    }

    async fn send_message_before(
        &self,
        message: &EmailMessage,
        deliver_before: DateTime<Utc>,
    ) -> Result<SendResult, EmailError> {
        self.send(message, Some(deliver_before)).await
    }
}
