//! Delivery of validated [`EmailCommand`]s, independent of the transport.
//!
//! Producers such as Identity hand commands to an [`EmailCommandSender`]. The
//! monolith delivers them in-process through [`DirectEmailDelivery`], which
//! renders the template and sends it with any [`EmailSender`] (SMTP locally,
//! Scaleway Transactional Email in production). [`EmailClient`] remains the
//! sender for a remote email service reached over gRPC.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    EmailClient, EmailClientError, EmailCommand, EmailFailureClass, EmailReceipt,
    trait_def::{EmailAddress, EmailMessage, EmailSender},
};

/// Correlates one delivery attempt across SMTP `Message-ID` and provider logs.
const JOB_ID_HEADER: &str = "x-nvbes-email-job-id";

#[async_trait]
pub trait EmailCommandSender: Send + Sync {
    /// `Unavailable` means retryable; every other error is final for the command.
    async fn send(&self, command: EmailCommand) -> Result<EmailReceipt, EmailClientError>;
}

#[async_trait]
impl EmailCommandSender for EmailClient {
    async fn send(&self, command: EmailCommand) -> Result<EmailReceipt, EmailClientError> {
        EmailClient::send(self, command).await
    }
}

pub struct DirectEmailDelivery {
    sender: Arc<dyn EmailSender>,
    from: EmailAddress,
}

impl DirectEmailDelivery {
    pub fn new(sender: Arc<dyn EmailSender>, from: EmailAddress) -> Self {
        Self { sender, from }
    }

    fn message(&self, command: &EmailCommand, job_id: Uuid) -> EmailMessage {
        let rendered = command.template.render();
        EmailMessage {
            from: self.from.clone(),
            to: vec![EmailAddress {
                email: command.recipient.email.clone(),
                name: command.recipient.name.clone(),
            }],
            subject: rendered.subject,
            text_body: Some(rendered.text_body),
            html_body: Some(rendered.html_body),
            headers: vec![(JOB_ID_HEADER.to_owned(), job_id.to_string())],
        }
    }
}

#[async_trait]
impl EmailCommandSender for DirectEmailDelivery {
    async fn send(&self, command: EmailCommand) -> Result<EmailReceipt, EmailClientError> {
        command
            .validate(Utc::now())
            .map_err(EmailClientError::InvalidCommand)?;
        let message = self.message(&command, Uuid::new_v4());
        let sent = self
            .sender
            .send_message_before(&message, command.deliver_before)
            .await
            .map_err(|error| match error.failure_class() {
                EmailFailureClass::Permanent => EmailClientError::Rejected,
                EmailFailureClass::Transient | EmailFailureClass::Ambiguous => {
                    EmailClientError::Unavailable
                }
            })?;
        if sent.provider_email_id.is_empty() {
            return Err(EmailClientError::Protocol);
        }
        Ok(EmailReceipt {
            message_id: sent.provider_email_id,
            accepted_at: Utc::now(),
            deliver_before: command.deliver_before,
            duplicate: false,
        })
    }
}

#[cfg(test)]
#[path = "delivery.tests.rs"]
mod tests;
