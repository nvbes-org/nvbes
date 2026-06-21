use async_trait::async_trait;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{debug, error};

use crate::error::EmailError;
use crate::trait_def::{EmailAddress, EmailMessage, EmailSender, SendResult};

#[derive(Debug, Clone)]
pub struct SmtpEmailConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub starttls: bool,
}

#[derive(Clone)]
pub struct SmtpEmailSender {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpEmailSender {
    pub fn new(config: SmtpEmailConfig) -> Result<Self, EmailError> {
        let mut builder = if config.starttls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
        }
        .port(config.port);

        if let (Some(username), Some(password)) = (config.username, config.password) {
            builder = builder.credentials(Credentials::new(username, password));
        }

        Ok(Self {
            mailer: builder.build(),
        })
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError> {
        let mut builder = Message::builder()
            .from(mailbox(&message.from)?)
            .subject(&message.subject);

        for recipient in &message.to {
            builder = builder.to(mailbox(recipient)?);
        }

        for (name, value) in &message.headers {
            if name.eq_ignore_ascii_case("reply-to") {
                let reply_to = EmailAddress {
                    email: value.clone(),
                    name: None,
                };
                builder = builder.reply_to(mailbox(&reply_to)?);
            }
        }

        let email = match (&message.text_body, &message.html_body) {
            (Some(text), Some(html)) => builder.multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text.clone()))
                    .singlepart(SinglePart::html(html.clone())),
            )?,
            (Some(text), None) => builder.singlepart(SinglePart::plain(text.clone()))?,
            (None, Some(html)) => builder.singlepart(SinglePart::html(html.clone()))?,
            (None, None) => builder.singlepart(SinglePart::plain(String::new()))?,
        };

        debug!(
            to = ?message.to.iter().map(|address| &address.email).collect::<Vec<_>>(),
            subject = %message.subject,
            "SMTP email: sending"
        );

        let response = self.mailer.send(email).await.map_err(|error| {
            error!(error = %error, "SMTP email: send failed");
            error
        })?;

        Ok(SendResult {
            provider_email_id: response.message().collect::<Vec<_>>().join(" "),
        })
    }
}

fn mailbox(address: &EmailAddress) -> Result<Mailbox, EmailError> {
    Ok(Mailbox::new(address.name.clone(), address.email.parse()?))
}
