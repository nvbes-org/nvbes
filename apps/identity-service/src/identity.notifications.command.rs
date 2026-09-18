use chrono::{DateTime, Duration, Utc};
use nvbes_email::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailCommandError, EmailIdempotencyKey,
    EmailRecipient, EmailRequestContext, EmailTemplate,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("unsupported Identity security event")]
    UnsupportedEvent,
    #[error("invalid Identity security event time")]
    InvalidTime,
    #[error("invalid Identity notification command")]
    InvalidCommand(#[from] EmailCommandError),
}

/// Build from a trusted outbox row and a verified recipient snapshot, never an HTTP body.
/// The event id and occurrence time keep retries identical, including the deadline.
pub fn recovery_command(
    event_id: Uuid,
    principal_id: Uuid,
    event_type: &str,
    occurred_at: DateTime<Utc>,
    recipient: String,
    now: DateTime<Utc>,
) -> Result<EmailCommand, NotificationError> {
    let event = match event_type {
        "identity.mfa_recovery_codes_generated" => AccountSecurityEvent::MfaRecoveryCodesGenerated,
        "identity.mfa_recovery_started" => AccountSecurityEvent::MfaRecoveryStarted,
        "identity.mfa_recovered" => AccountSecurityEvent::MfaRecovered,
        "identity.mfa_recovery_cancelled" => AccountSecurityEvent::MfaRecoveryCancelled,
        "identity.password_recovered" => AccountSecurityEvent::PasswordRecovered,
        _ => return Err(NotificationError::UnsupportedEvent),
    };
    let deliver_before = occurred_at
        .checked_add_signed(Duration::hours(24))
        .ok_or(NotificationError::InvalidTime)?;
    if occurred_at > now || deliver_before <= now {
        return Err(NotificationError::InvalidTime);
    }
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: event_id.to_string(),
            correlation_id: event_id.to_string(),
            actor_principal_id: principal_id.to_string(),
        },
        producer: "identity-service".into(),
        idempotency_key: EmailIdempotencyKey::new(format!(
            "identity:{}-notification:{event_id}:{}",
            if event_type == "identity.password_recovered" {
                "password"
            } else {
                "mfa"
            },
            hex::encode(Sha256::digest(recipient.as_bytes()))
        ))?,
        recipient: EmailRecipient {
            email: recipient,
            name: None,
        },
        category: EmailCategory::AccountSecurity,
        template: EmailTemplate::AccountSecurityV1 {
            event,
            affected_email: None,
            previous_email: None,
            security_url: None,
        },
        deliver_before,
    };
    command.validate(now)?;
    Ok(command)
}

#[cfg(test)]
#[path = "identity.notifications.command.tests.rs"]
mod tests;
