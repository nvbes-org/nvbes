use async_trait::async_trait;
use lettre::message::{
    Mailbox, Message as LettreMessage, MultiPart, SinglePart,
    header::{HeaderName, HeaderValue},
};
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
        let email = build_message(message)?;

        debug!(recipient_count = message.to.len(), "SMTP email: sending");

        let response = match self.mailer.send(email).await {
            Ok(response) => response,
            Err(error) => {
                let classified = EmailError::Smtp(error);
                error!(
                    error_code = classified.safe_code(),
                    error_class = ?classified.failure_class(),
                    "SMTP email: send failed"
                );
                return Err(classified);
            }
        };
        let provider_email_id = effective_message_id(message)
            .unwrap_or_else(|| response.message().collect::<Vec<_>>().join(" "));

        Ok(SendResult { provider_email_id })
    }
}

fn build_message(message: &EmailMessage) -> Result<LettreMessage, EmailError> {
    let mut builder = Message::builder()
        .from(mailbox(&message.from)?)
        .subject(&message.subject);

    if header_value(message, "message-id").is_none()
        && let Some(message_id) = effective_message_id(message)
    {
        builder = builder.message_id(Some(message_id));
    }

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
        } else if name.eq_ignore_ascii_case("message-id") {
            builder = builder.message_id(Some(value.clone()));
        } else if name.to_ascii_lowercase().starts_with("x-") {
            let name = custom_header_name(name)?;
            builder = builder.raw_header(HeaderValue::new(name, value.clone()));
        } else {
            return Err(EmailError::Config(format!(
                "unsupported email header: {name}"
            )));
        }
    }

    match (&message.text_body, &message.html_body) {
        (Some(text), Some(html)) => Ok(builder.multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(text.clone()))
                .singlepart(SinglePart::html(html.clone())),
        )?),
        (Some(text), None) => Ok(builder.singlepart(SinglePart::plain(text.clone()))?),
        (None, Some(html)) => Ok(builder.singlepart(SinglePart::html(html.clone()))?),
        (None, None) => Ok(builder.singlepart(SinglePart::plain(String::new()))?),
    }
}

fn custom_header_name(value: &str) -> Result<HeaderName, EmailError> {
    let valid = !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        });
    if !valid {
        return Err(EmailError::Config(
            "invalid custom email header name".to_string(),
        ));
    }
    HeaderName::new_from_ascii(value.to_string())
        .map_err(|_| EmailError::Config("invalid custom email header name".to_string()))
}

fn header_value<'a>(message: &'a EmailMessage, expected: &str) -> Option<&'a str> {
    message
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(expected))
        .map(|(_, value)| value.as_str())
}

fn effective_message_id(message: &EmailMessage) -> Option<String> {
    header_value(message, "message-id")
        .map(String::from)
        .or_else(|| {
            header_value(message, "x-nvbes-email-job-id")
                .map(|job_id| format!("<account-job-{job_id}@notify.nvbes.eu>"))
        })
}

fn mailbox(address: &EmailAddress) -> Result<Mailbox, EmailError> {
    Ok(Mailbox::new(address.name.clone(), address.email.parse()?))
}

#[cfg(test)]
mod tests {
    use super::build_message;
    use crate::{EmailAddress, EmailMessage};

    fn message(headers: Vec<(String, String)>) -> EmailMessage {
        EmailMessage {
            from: EmailAddress {
                email: "sender@example.test".to_string(),
                name: Some("Sender".to_string()),
            },
            to: vec![EmailAddress {
                email: "recipient@example.test".to_string(),
                name: None,
            }],
            subject: "Delivery contract".to_string(),
            text_body: Some("plain text".to_string()),
            html_body: Some("<p>html</p>".to_string()),
            headers,
        }
    }

    #[test]
    fn smtp_serializes_stable_message_and_job_identifiers() {
        let message_id = "<account-job-00000000-0000-0000-0000-000000000001@notify.nvbes.eu>";
        let formatted = build_message(&message(vec![
            ("Message-ID".to_string(), message_id.to_string()),
            (
                "X-Nvbes-Email-Job-Id".to_string(),
                "00000000-0000-0000-0000-000000000001".to_string(),
            ),
        ]))
        .expect("message should build")
        .formatted();
        let formatted = String::from_utf8(formatted).expect("message should be UTF-8");

        assert!(formatted.contains(&format!("Message-ID: {message_id}\r\n")));
        assert!(
            formatted.contains("X-Nvbes-Email-Job-Id: 00000000-0000-0000-0000-000000000001\r\n")
        );
        assert!(formatted.contains("Subject: Delivery contract"));
        assert!(formatted.contains("plain text") || formatted.contains("<p>html</p>"));
    }

    #[test]
    fn smtp_rejects_unapproved_standard_headers() {
        let error = build_message(&message(vec![(
            "Bcc".to_string(),
            "hidden@example.test".to_string(),
        )]))
        .expect_err("protected headers must not be overridden");

        assert_eq!(error.safe_code(), "email_configuration");
    }

    #[test]
    fn smtp_custom_header_values_cannot_inject_a_second_header() {
        let formatted = build_message(&message(vec![(
            "X-Nvbes-Email-Job-Id".to_string(),
            "job-id\r\nBcc: hidden@example.test".to_string(),
        )]))
        .expect("safe encoder should build the message")
        .formatted();
        let formatted = String::from_utf8(formatted).expect("message should be UTF-8");

        assert!(!formatted.contains("\r\nBcc: hidden@example.test"));
    }

    #[test]
    fn smtp_builds_text_html_empty_and_reply_to_messages() {
        for bodies in [
            (Some("text".to_string()), None),
            (None, Some("<p>html</p>".to_string())),
            (None, None),
        ] {
            let mut value = message(Vec::new());
            value.text_body = bodies.0;
            value.html_body = bodies.1;
            assert!(!build_message(&value).unwrap().formatted().is_empty());
        }

        let formatted = build_message(&message(vec![(
            "Reply-To".to_string(),
            "support@example.test".to_string(),
        )]))
        .unwrap()
        .formatted();
        assert!(
            String::from_utf8(formatted)
                .unwrap()
                .contains("Reply-To: support@example.test")
        );
    }

    #[test]
    fn smtp_rejects_invalid_mailboxes_and_custom_header_names() {
        let mut invalid_from = message(Vec::new());
        invalid_from.from.email = "invalid".to_string();
        assert_eq!(
            build_message(&invalid_from).unwrap_err().safe_code(),
            "email_address"
        );

        let invalid_reply_to = message(vec![("Reply-To".to_string(), "invalid".to_string())]);
        assert_eq!(
            build_message(&invalid_reply_to).unwrap_err().safe_code(),
            "email_address"
        );

        let invalid_header = message(vec![("X-Bad\nName".to_string(), "value".to_string())]);
        assert_eq!(
            build_message(&invalid_header).unwrap_err().safe_code(),
            "email_configuration"
        );
    }
}

#[cfg(test)]
#[path = "smtp.protocol.tests.rs"]
mod protocol_tests;
