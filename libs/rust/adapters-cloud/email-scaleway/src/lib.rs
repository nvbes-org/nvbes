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

    #[cfg(test)]
    fn for_test(api_url: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .expect("test email client should initialize"),
            api_url,
            secret_key: "test-secret".to_string(),
            project_id: "test-project".to_string(),
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

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        http::{HeaderMap, StatusCode},
        routing::post,
    };
    use chrono::{Duration, Utc};
    use nvbes_email::{EmailAddress, EmailFailureClass, EmailMessage, EmailSender};
    use serde_json::{Value, json};

    use super::ScalewayEmailClient;

    async fn accepted(headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
        assert_eq!(headers.get("x-auth-token").unwrap(), "test-secret");
        assert_eq!(body["project_id"], "test-project");
        assert_eq!(body["to"][0]["email"], "recipient@nvbes.fr");
        assert_eq!(body["additional_headers"][0]["key"], "Message-ID");
        assert!(body["send_before"].as_str().is_some());
        Json(json!({ "emails": [{ "id": "provider-email-1" }] }))
    }

    async fn rejected() -> StatusCode {
        StatusCode::TOO_MANY_REQUESTS
    }

    async fn server(router: Router) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://{address}/emails"), task)
    }

    fn message() -> EmailMessage {
        EmailMessage {
            from: EmailAddress {
                email: "no-reply@notify.nvbes.eu".to_string(),
                name: Some("nvbes".to_string()),
            },
            to: vec![EmailAddress {
                email: "recipient@nvbes.fr".to_string(),
                name: None,
            }],
            subject: "Security notification".to_string(),
            text_body: Some("Security notification".to_string()),
            html_body: Some("<p>Security notification</p>".to_string()),
            headers: vec![(
                "Message-ID".to_string(),
                "<stable@notify.nvbes.eu>".to_string(),
            )],
        }
    }

    #[tokio::test]
    async fn adapter_sends_multipart_payload_deadline_and_stable_headers() {
        let (endpoint, task) = server(Router::new().route("/emails", post(accepted))).await;
        let client = ScalewayEmailClient::for_test(endpoint);
        let result = client
            .send_message_before(&message(), Utc::now() + Duration::minutes(5))
            .await
            .unwrap();
        assert_eq!(result.provider_email_id, "provider-email-1");
        task.abort();
    }

    #[tokio::test]
    async fn adapter_classifies_rate_limits_as_transient() {
        let (endpoint, task) = server(Router::new().route("/emails", post(rejected))).await;
        let error = ScalewayEmailClient::for_test(endpoint)
            .send_message(&message())
            .await
            .unwrap_err();
        assert_eq!(error.failure_class(), EmailFailureClass::Transient);
        task.abort();
    }
}
