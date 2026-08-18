use async_trait::async_trait;

use crate::error::EmailError;

#[derive(Debug, Clone)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SendResult {
    pub provider_email_id: String,
}

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError>;

    async fn send_message_before(
        &self,
        message: &EmailMessage,
        _deliver_before: chrono::DateTime<chrono::Utc>,
    ) -> Result<SendResult, EmailError> {
        self.send_message(message).await
    }
}
