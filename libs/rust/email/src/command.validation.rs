use chrono::{DateTime, Duration, Utc};

use super::{EmailCommand, EmailCommandError, EmailTemplate};

const MAX_PRODUCER_LENGTH: usize = 64;
const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 200;
const MAX_TEXT_FIELD_LENGTH: usize = 200;
const MAX_EMAIL_LENGTH: usize = 320;
const MAX_URL_LENGTH: usize = 4096;

pub(super) fn validate_idempotency_key(value: &str) -> Result<(), EmailCommandError> {
    validate_identifier("idempotency_key", value, MAX_IDEMPOTENCY_KEY_LENGTH)
}

impl EmailCommand {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), EmailCommandError> {
        validate_identifier("producer", &self.producer, MAX_PRODUCER_LENGTH)?;
        validate_email(&self.recipient.email)?;
        validate_optional_text("recipient.name", self.recipient.name.as_deref())?;
        if self.deliver_before <= now {
            return Err(EmailCommandError::field("deliver_before"));
        }
        if self.category != self.template.category() {
            return Err(EmailCommandError::field("category"));
        }
        self.template.validate(now, self.deliver_before)
    }
}

impl EmailTemplate {
    fn validate(
        &self,
        now: DateTime<Utc>,
        deliver_before: DateTime<Utc>,
    ) -> Result<(), EmailCommandError> {
        match self {
            Self::EmailVerificationV1 {
                user_name,
                verification_url,
                credential_expires_at,
                timezone,
            } => {
                timezone
                    .parse::<chrono_tz::Tz>()
                    .map_err(|_| EmailCommandError::field("template.timezone"))?;
                validate_credential(
                    user_name,
                    verification_url,
                    *credential_expires_at,
                    deliver_before,
                    now,
                    Duration::hours(24),
                )
            }
            Self::PasswordResetV1 {
                user_name,
                reset_url,
                credential_expires_at,
            } => validate_credential(
                user_name,
                reset_url,
                *credential_expires_at,
                deliver_before,
                now,
                Duration::hours(2),
            ),
            Self::PasswordChangeCodeV1 {
                user_name,
                code,
                credential_expires_at,
            } => {
                validate_text("template.user_name", user_name)?;
                if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(EmailCommandError::field("template.code"));
                }
                validate_credential_expiry(
                    *credential_expires_at,
                    deliver_before,
                    now,
                    Duration::minutes(15),
                )
            }
            Self::AccountSecurityV1 {
                affected_email,
                previous_email,
                security_url,
                ..
            } => {
                validate_optional_email("template.affected_email", affected_email.as_deref())?;
                validate_optional_email("template.previous_email", previous_email.as_deref())?;
                validate_optional_https_url("template.security_url", security_url.as_deref())
            }
            Self::BillingReceiptV1 {
                customer_name,
                amount_minor,
                currency,
                invoice_url,
                provider_name,
            } => {
                validate_optional_text("template.customer_name", customer_name.as_deref())?;
                validate_money(*amount_minor, currency)?;
                validate_optional_https_url("template.invoice_url", invoice_url.as_deref())?;
                validate_optional_text("template.provider_name", provider_name.as_deref())
            }
            Self::BillingPaymentFailureV1 {
                customer_name,
                amount_minor,
                currency,
                billing_portal_url,
                invoice_url,
                provider_name,
            } => {
                validate_optional_text("template.customer_name", customer_name.as_deref())?;
                validate_money(*amount_minor, currency)?;
                validate_optional_https_url(
                    "template.billing_portal_url",
                    billing_portal_url.as_deref(),
                )?;
                validate_optional_https_url("template.invoice_url", invoice_url.as_deref())?;
                validate_optional_text("template.provider_name", provider_name.as_deref())
            }
            Self::AccessReviewReminderV1 {
                reviewer_name,
                campaign_name,
                review_url,
                review_due_at: _,
            } => {
                validate_text("template.reviewer_name", reviewer_name)?;
                validate_text("template.campaign_name", campaign_name)?;
                validate_https_url("template.review_url", review_url)?;
                if deliver_before - now > Duration::hours(24) {
                    return Err(EmailCommandError::field("deliver_before"));
                }
                Ok(())
            }
        }
    }
}

fn validate_credential(
    name: &str,
    url: &str,
    expiry: DateTime<Utc>,
    deadline: DateTime<Utc>,
    now: DateTime<Utc>,
    max_ttl: Duration,
) -> Result<(), EmailCommandError> {
    validate_text("template.user_name", name)?;
    validate_https_url("template.url", url)?;
    validate_credential_expiry(expiry, deadline, now, max_ttl)
}

fn validate_credential_expiry(
    expiry: DateTime<Utc>,
    deadline: DateTime<Utc>,
    now: DateTime<Utc>,
    max_ttl: Duration,
) -> Result<(), EmailCommandError> {
    if expiry != deadline || expiry <= now || expiry - now > max_ttl {
        return Err(EmailCommandError::field("template.credential_expires_at"));
    }
    Ok(())
}

fn validate_identifier(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), EmailCommandError> {
    if value.is_empty()
        || value.len() > max
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
    {
        return Err(EmailCommandError::field(field));
    }
    Ok(())
}

fn validate_email(value: &str) -> Result<(), EmailCommandError> {
    if value.len() > MAX_EMAIL_LENGTH {
        return Err(EmailCommandError::field("recipient.email"));
    }
    value
        .parse::<lettre::Address>()
        .map(|_| ())
        .map_err(|_| EmailCommandError::field("recipient.email"))
}

fn validate_optional_email(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), EmailCommandError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > MAX_EMAIL_LENGTH || value.parse::<lettre::Address>().is_err() {
        return Err(EmailCommandError::field(field));
    }
    Ok(())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), EmailCommandError> {
    if value.trim().is_empty()
        || value.len() > MAX_TEXT_FIELD_LENGTH
        || value.contains(['\r', '\n'])
    {
        return Err(EmailCommandError::field(field));
    }
    Ok(())
}

fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), EmailCommandError> {
    match value {
        Some(value) => validate_text(field, value),
        None => Ok(()),
    }
}

fn validate_money(amount_minor: i64, currency: &str) -> Result<(), EmailCommandError> {
    if amount_minor < 0
        || currency.len() != 3
        || !currency.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(EmailCommandError::field("template.money"));
    }
    Ok(())
}

fn validate_https_url(field: &'static str, value: &str) -> Result<(), EmailCommandError> {
    if value.len() > MAX_URL_LENGTH {
        return Err(EmailCommandError::field(field));
    }
    let url = reqwest::Url::parse(value).map_err(|_| EmailCommandError::field(field))?;
    let loopback_http =
        url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if (url.scheme() != "https" && !loopback_http) || url.host_str().is_none() {
        return Err(EmailCommandError::field(field));
    }
    Ok(())
}

fn validate_optional_https_url(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), EmailCommandError> {
    match value {
        Some(value) => validate_https_url(field, value),
        None => Ok(()),
    }
}
