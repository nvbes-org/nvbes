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

    fn message(to: Vec<EmailAddress>, headers: Vec<(String, String)>) -> EmailMessage {
        EmailMessage {
            from: EmailAddress {
                email: "sender@example.test".to_string(),
                name: None,
            },
            to,
            subject: "Subject".to_string(),
            text_body: Some("Body".to_string()),
            html_body: Some("<p>Body</p>".to_string()),
            headers,
        }
    }

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

    #[tokio::test]
    async fn mock_captures_message_content_and_exposes_counts() {
        let sender = MockEmailSender::default();
        let recipient = EmailAddress {
            email: "recipient@example.test".to_string(),
            name: Some("Ada".to_string()),
        };
        let result = sender
            .send_message_before(
                &message(vec![recipient.clone()], Vec::new()),
                chrono::Utc::now(),
            )
            .await
            .expect("mock delivery");

        assert_eq!(result.provider_email_id, "mock-recipient@example.test");
        assert_eq!(sender.sent_count().await, 1);
        let captured = sender.sent_emails().await;
        assert_eq!(captured[0].to[0].email, recipient.email);
        assert_eq!(captured[0].subject, "Subject");
        assert_eq!(captured[0].html_body.as_deref(), Some("<p>Body</p>"));
        assert!(captured[0].headers.is_empty());
    }

    #[tokio::test]
    async fn mock_derives_provider_ids_from_job_or_empty_recipient() {
        let sender = MockEmailSender::new();
        let job = sender
            .send_message(&message(
                Vec::new(),
                vec![(
                    "X-Nvbes-Email-Job-Id".to_string(),
                    "00000000-0000-0000-0000-000000000042".to_string(),
                )],
            ))
            .await
            .unwrap();
        assert_eq!(
            job.provider_email_id,
            "<account-job-00000000-0000-0000-0000-000000000042@notify.nvbes.eu>"
        );

        let unknown = sender
            .send_message(&message(Vec::new(), Vec::new()))
            .await
            .unwrap();
        assert_eq!(unknown.provider_email_id, "mock-unknown");
    }
}
