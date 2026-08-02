use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailRequestContext {
    pub request_id: String,
    pub correlation_id: String,
    pub actor_principal_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailRecipient {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailIdempotencyKey(String);

impl EmailIdempotencyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, EmailCommandError> {
        let value = value.into();
        validation::validate_idempotency_key(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailCategory {
    Credential,
    AccountSecurity,
    Billing,
    Reminder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountSecurityEvent {
    EmailAdded,
    PrimaryEmailChanged,
    AccountRecovered,
    RecoveryReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailTemplate {
    EmailVerificationV1 {
        user_name: String,
        verification_url: String,
        credential_expires_at: DateTime<Utc>,
    },
    PasswordResetV1 {
        user_name: String,
        reset_url: String,
        credential_expires_at: DateTime<Utc>,
    },
    PasswordChangeCodeV1 {
        user_name: String,
        code: String,
        credential_expires_at: DateTime<Utc>,
    },
    AccountSecurityV1 {
        event: AccountSecurityEvent,
        affected_email: Option<String>,
        previous_email: Option<String>,
        security_url: Option<String>,
    },
    BillingReceiptV1 {
        customer_name: Option<String>,
        amount_minor: i64,
        currency: String,
        invoice_url: Option<String>,
        provider_name: Option<String>,
    },
    BillingPaymentFailureV1 {
        customer_name: Option<String>,
        amount_minor: i64,
        currency: String,
        billing_portal_url: Option<String>,
        invoice_url: Option<String>,
        provider_name: Option<String>,
    },
    AccessReviewReminderV1 {
        reviewer_name: String,
        campaign_name: String,
        review_url: String,
        review_due_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailCommand {
    pub context: EmailRequestContext,
    pub producer: String,
    pub idempotency_key: EmailIdempotencyKey,
    pub recipient: EmailRecipient,
    pub category: EmailCategory,
    pub template: EmailTemplate,
    pub deliver_before: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailReceipt {
    pub message_id: String,
    pub accepted_at: DateTime<Utc>,
    pub deliver_before: DateTime<Utc>,
    pub duplicate: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid email command: {field}")]
pub struct EmailCommandError {
    field: &'static str,
}

impl EmailCommandError {
    pub(crate) fn field(field: &'static str) -> Self {
        Self { field }
    }

    pub fn field_name(&self) -> &'static str {
        self.field
    }
}

impl EmailTemplate {
    pub fn name_and_version(&self) -> (&'static str, i16) {
        let name = match self {
            Self::EmailVerificationV1 { .. } => "email_verification",
            Self::PasswordResetV1 { .. } => "password_reset",
            Self::PasswordChangeCodeV1 { .. } => "password_change_code",
            Self::AccountSecurityV1 { .. } => "account_security",
            Self::BillingReceiptV1 { .. } => "billing_receipt",
            Self::BillingPaymentFailureV1 { .. } => "billing_payment_failure",
            Self::AccessReviewReminderV1 { .. } => "access_review_reminder",
        };
        (name, 1)
    }

    pub fn category(&self) -> EmailCategory {
        match self {
            Self::EmailVerificationV1 { .. }
            | Self::PasswordResetV1 { .. }
            | Self::PasswordChangeCodeV1 { .. } => EmailCategory::Credential,
            Self::AccountSecurityV1 { .. } => EmailCategory::AccountSecurity,
            Self::BillingReceiptV1 { .. } | Self::BillingPaymentFailureV1 { .. } => {
                EmailCategory::Billing
            }
            Self::AccessReviewReminderV1 { .. } => EmailCategory::Reminder,
        }
    }
}

impl EmailCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Credential => "credential",
            Self::AccountSecurity => "account_security",
            Self::Billing => "billing",
            Self::Reminder => "reminder",
        }
    }
}

pub(crate) fn timestamp(value: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

pub(crate) fn datetime(
    value: prost_types::Timestamp,
    field: &'static str,
) -> Result<DateTime<Utc>, EmailCommandError> {
    DateTime::from_timestamp(value.seconds, value.nanos as u32)
        .ok_or_else(|| EmailCommandError::field(field))
}

#[path = "command.validation.rs"]
mod validation;

#[path = "command.proto.decode.rs"]
mod proto_decode;
#[path = "command.proto.encode.rs"]
mod proto_encode;

#[cfg(test)]
#[path = "command.tests.rs"]
mod tests;
