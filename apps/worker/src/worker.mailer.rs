//! Builds the in-process email delivery used by `identity-emails`.
//!
//! `smtp` targets Mailpit locally or any relay; `scaleway-tem` targets
//! Scaleway Transactional Email; `test-capture` writes messages to a private
//! directory for end-to-end tests and only exists in development. Outside
//! development the provider is explicit and SMTP must use STARTTLS.

use std::sync::Arc;

use nvbes_email::{
    DirectEmailDelivery, EmailAddress, EmailSender, SmtpEmailConfig, SmtpEmailSender,
    TestCaptureEmailSender,
};
use nvbes_email_scaleway::ScalewayEmailClient;

use crate::config::{ConfigError, Lookup, present};

const PROVIDER: &str = "NVBES_EMAIL_PROVIDER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Smtp,
    ScalewayTem,
    TestCapture,
}

pub fn provider(lookup: Lookup<'_>, development: bool) -> Result<Provider, ConfigError> {
    match present(lookup, PROVIDER).as_deref() {
        Some("smtp") => Ok(Provider::Smtp),
        Some("scaleway-tem") => Ok(Provider::ScalewayTem),
        Some("test-capture") if development => Ok(Provider::TestCapture),
        Some(other) => Err(ConfigError::Invalid(format!(
            "{PROVIDER}: unsupported provider {other}"
        ))),
        None if development => Ok(Provider::Smtp),
        None => Err(ConfigError::Missing(PROVIDER)),
    }
}

pub fn from_lookup(
    lookup: Lookup<'_>,
    development: bool,
) -> Result<DirectEmailDelivery, ConfigError> {
    let from = EmailAddress {
        email: match present(lookup, "NVBES_EMAIL_FROM_EMAIL") {
            Some(email) => email,
            None if development => "no-reply@notify.nvbes.eu".into(),
            None => return Err(ConfigError::Missing("NVBES_EMAIL_FROM_EMAIL")),
        },
        name: present(lookup, "NVBES_EMAIL_FROM_NAME"),
    };
    let sender: Arc<dyn EmailSender> = match provider(lookup, development)? {
        Provider::Smtp => Arc::new(smtp(lookup, development)?),
        Provider::ScalewayTem => Arc::new(ScalewayEmailClient::new(
            required(lookup, "NVBES_SCALEWAY_EMAIL_SECRET_KEY")?,
            required(lookup, "NVBES_SCALEWAY_EMAIL_PROJECT_ID")?,
            &present(lookup, "NVBES_SCALEWAY_EMAIL_REGION").unwrap_or_else(|| "fr-par".into()),
        )),
        Provider::TestCapture => Arc::new(
            TestCaptureEmailSender::new(required(lookup, "NVBES_EMAIL_TEST_CAPTURE_DIR")?)
                .map_err(|error| ConfigError::Invalid(format!("test capture: {error}")))?,
        ),
    };
    Ok(DirectEmailDelivery::new(sender, from))
}

fn smtp(lookup: Lookup<'_>, development: bool) -> Result<SmtpEmailSender, ConfigError> {
    let host = match present(lookup, "NVBES_SMTP_HOST") {
        Some(host) => host,
        None if development => "127.0.0.1".into(),
        None => return Err(ConfigError::Missing("NVBES_SMTP_HOST")),
    };
    let port = match present(lookup, "NVBES_SMTP_PORT") {
        Some(port) => port
            .parse()
            .map_err(|_| ConfigError::Invalid("NVBES_SMTP_PORT".into()))?,
        None if development => 1025,
        None => return Err(ConfigError::Missing("NVBES_SMTP_PORT")),
    };
    let starttls = match present(lookup, "NVBES_SMTP_STARTTLS").as_deref() {
        Some("true") => true,
        Some("false") | None => false,
        Some(_) => return Err(ConfigError::Invalid("NVBES_SMTP_STARTTLS".into())),
    };
    if !development && !starttls {
        return Err(ConfigError::Invalid(
            "NVBES_SMTP_STARTTLS must be true outside development".into(),
        ));
    }
    let username = present(lookup, "NVBES_SMTP_USERNAME");
    let password = present(lookup, "NVBES_SMTP_PASSWORD");
    if username.is_some() != password.is_some() {
        return Err(ConfigError::Invalid(
            "NVBES_SMTP_USERNAME and NVBES_SMTP_PASSWORD go together".into(),
        ));
    }
    SmtpEmailSender::new(SmtpEmailConfig {
        host,
        port,
        username,
        password,
        starttls,
    })
    .map_err(|error| ConfigError::Invalid(format!("SMTP: {error}")))
}

fn required(lookup: Lookup<'_>, name: &'static str) -> Result<String, ConfigError> {
    present(lookup, name).ok_or(ConfigError::Missing(name))
}

#[cfg(test)]
#[path = "worker.mailer.tests.rs"]
mod tests;
