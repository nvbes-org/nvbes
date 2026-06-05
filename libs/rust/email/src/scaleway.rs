use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::error::EmailError;
use crate::trait_def::{EmailMessage, EmailSender, SendResult};

#[derive(Clone)]
pub struct ScalewayEmailClient {
    client: reqwest::Client,
    api_url: String,
    secret_key: String,
    project_id: String,
}

#[derive(Debug, Deserialize)]
struct ScalewayEmailResponse {
    email_id: String,
}

#[derive(Debug, Serialize)]
struct ScalewayAddress {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScalewayHeader {
    header: String,
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
}

#[async_trait]
impl EmailSender for ScalewayEmailClient {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError> {
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
            additional_headers: message
                .headers
                .iter()
                .map(|(h, v)| ScalewayHeader {
                    header: h.clone(),
                    value: v.clone(),
                })
                .collect(),
        };

        debug!(
            to = ?request.to.iter().map(|a| &a.email).collect::<Vec<_>>(),
            subject = %request.subject,
            "ScalewayEmail: sending"
        );

        let response = self
            .client
            .post(&self.api_url)
            .header("X-Session-Token", &self.secret_key)
            .json(&request);
        let response = nvbes_core::trace_context::with_fresh_trace_headers(response)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let body_size_bytes = body.len();
            error!(
                status = %status,
                body_size_bytes,
                "ScalewayEmail: API error"
            );
            return Err(EmailError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let body = response.text().await?;
        let resp: ScalewayEmailResponse = serde_json::from_str(&body)?;
        debug!(
            email_id = %resp.email_id,
            "ScalewayEmail: sent successfully"
        );
        Ok(SendResult {
            provider_email_id: resp.email_id,
        })
    }
}
