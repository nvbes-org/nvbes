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
        info!(
            to = ?message.to.iter().map(|a| &a.email).collect::<Vec<_>>(),
            subject = %message.subject,
            "MockEmail: email sent (not delivered)"
        );

        self.sent.lock().await.push(SentEmail {
            to: message.to.clone(),
            subject: message.subject.clone(),
            html_body: message.html_body.clone(),
            headers: message.headers.clone(),
        });

        Ok(SendResult {
            provider_email_id: format!(
                "mock-{}",
                message
                    .to
                    .first()
                    .map(|a| &a.email)
                    .unwrap_or(&"unknown".to_string())
            ),
        })
    }
}
