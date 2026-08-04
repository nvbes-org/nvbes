use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::info;

use crate::error::EmailError;
use crate::trait_def::{EmailAddress, EmailMessage, EmailSender, SendResult};

#[derive(Debug, Clone)]
pub struct SentEmail {
    pub to: Vec<EmailAddress>,
    pub subject: String,
    pub html_body: Option<String>,
    pub headers: Vec<(String, String)>,
}

pub struct MockEmailSender {
    sent: Arc<Mutex<Vec<SentEmail>>>,
}

impl MockEmailSender {
    pub fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn sent_emails(&self) -> Vec<SentEmail> {
        self.sent.lock().await.clone()
    }

    pub async fn sent_count(&self) -> usize {
        self.sent.lock().await.len()
    }
}

impl Default for MockEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailSender for MockEmailSender {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError> {
        info!(recipient_count = message.to.len(), "mock email accepted");

        self.sent.lock().await.push(SentEmail {
            to: message.to.clone(),
            subject: message.subject.clone(),
            html_body: message.html_body.clone(),
            headers: message.headers.clone(),
        });

        let provider_email_id = message
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("message-id"))
            .map(|(_, value)| value.clone())
            .or_else(|| {
                message
                    .headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("x-nvbes-email-job-id"))
                    .map(|(_, value)| format!("<account-job-{value}@notify.nvbes.eu>"))
            })
            .unwrap_or_else(|| {
                format!(
                    "mock-{}",
                    message
                        .to
                        .first()
                        .map(|address| address.email.as_str())
                        .unwrap_or("unknown")
                )
            });

        Ok(SendResult { provider_email_id })
    }
}

#[cfg(test)]
mod tests {
    use super::MockEmailSender;
    use crate::{EmailAddress, EmailMessage, EmailSender};

    #[tokio::test]
    async fn mock_provider_uses_the_stable_message_id() {
        let sender = MockEmailSender::new();
        let message_id = "<account-job-00000000-0000-0000-0000-000000000001@notify.nvbes.eu>";
        let result = sender
            .send_message(&EmailMessage {
                from: EmailAddress {
                    email: "sender@example.test".to_string(),
                    name: None,
                },
                to: vec![EmailAddress {
                    email: "recipient@example.test".to_string(),
                    name: None,
                }],
                subject: "Subject".to_string(),
                text_body: Some("Body".to_string()),
                html_body: None,
                headers: vec![("Message-ID".to_string(), message_id.to_string())],
            })
            .await
            .expect("mock delivery should succeed");

        assert_eq!(result.provider_email_id, message_id);
    }
}
