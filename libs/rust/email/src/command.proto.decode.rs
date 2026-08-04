use crate::proto::nvbes::email::v1 as email_pb;

use super::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailCommandError, EmailIdempotencyKey,
    EmailRecipient, EmailRequestContext, EmailTemplate, datetime,
};

impl TryFrom<email_pb::SubmitEmailRequest> for EmailCommand {
    type Error = EmailCommandError;

    fn try_from(value: email_pb::SubmitEmailRequest) -> Result<Self, Self::Error> {
        let context = value
            .context
            .ok_or_else(|| EmailCommandError::field("context"))?;
        let recipient = value
            .recipient
            .ok_or_else(|| EmailCommandError::field("recipient"))?;
        let template = value
            .template
            .ok_or_else(|| EmailCommandError::field("template"))?
            .try_into()?;
        let category = email_pb::TransactionalEmailCategory::try_from(value.category)
            .map_err(|_| EmailCommandError::field("category"))?
            .try_into()?;
        let deliver_before = datetime(
            value
                .deliver_before
                .ok_or_else(|| EmailCommandError::field("deliver_before"))?,
            "deliver_before",
        )?;

        Ok(Self {
            context: EmailRequestContext {
                request_id: context.request_id,
                correlation_id: context.correlation_id,
                actor_principal_id: context.actor_principal_id,
            },
            producer: value.producer,
            idempotency_key: EmailIdempotencyKey::new(value.idempotency_key)?,
            recipient: EmailRecipient {
                email: recipient.email,
                name: recipient.name,
            },
            category,
            template,
            deliver_before,
        })
    }
}

impl TryFrom<email_pb::TransactionalEmailTemplate> for EmailTemplate {
    type Error = EmailCommandError;

    fn try_from(value: email_pb::TransactionalEmailTemplate) -> Result<Self, Self::Error> {
        use email_pb::transactional_email_template::Template;
        match value
            .template
            .ok_or_else(|| EmailCommandError::field("template"))?
        {
            Template::EmailVerificationV1(value) => Ok(Self::EmailVerificationV1 {
                user_name: value.user_name,
                verification_url: value.verification_url,
                credential_expires_at: required_time(value.credential_expires_at)?,
                timezone: if value.timezone.trim().is_empty() {
                    "UTC".to_string()
                } else {
                    value.timezone
                },
            }),
            Template::PasswordResetV1(value) => Ok(Self::PasswordResetV1 {
                user_name: value.user_name,
                reset_url: value.reset_url,
                credential_expires_at: required_time(value.credential_expires_at)?,
            }),
            Template::PasswordChangeCodeV1(value) => Ok(Self::PasswordChangeCodeV1 {
                user_name: value.user_name,
                code: value.code,
                credential_expires_at: required_time(value.credential_expires_at)?,
            }),
            Template::AccountSecurityV1(value) => Ok(Self::AccountSecurityV1 {
                event: email_pb::AccountSecurityEvent::try_from(value.event)
                    .map_err(|_| EmailCommandError::field("template.event"))?
                    .try_into()?,
                affected_email: value.affected_email,
                previous_email: value.previous_email,
                security_url: value.security_url,
            }),
            Template::BillingReceiptV1(value) => Ok(Self::BillingReceiptV1 {
                customer_name: value.customer_name,
                amount_minor: value.amount_minor,
                currency: value.currency,
                invoice_url: value.invoice_url,
                provider_name: value.provider_name,
            }),
            Template::BillingPaymentFailureV1(value) => Ok(Self::BillingPaymentFailureV1 {
                customer_name: value.customer_name,
                amount_minor: value.amount_minor,
                currency: value.currency,
                billing_portal_url: value.billing_portal_url,
                invoice_url: value.invoice_url,
                provider_name: value.provider_name,
            }),
            Template::AccessReviewReminderV1(value) => Ok(Self::AccessReviewReminderV1 {
                reviewer_name: value.reviewer_name,
                campaign_name: value.campaign_name,
                review_url: value.review_url,
                review_due_at: required_time(value.review_due_at)?,
            }),
        }
    }
}

impl TryFrom<email_pb::TransactionalEmailCategory> for EmailCategory {
    type Error = EmailCommandError;

    fn try_from(value: email_pb::TransactionalEmailCategory) -> Result<Self, Self::Error> {
        match value {
            email_pb::TransactionalEmailCategory::Credential => Ok(Self::Credential),
            email_pb::TransactionalEmailCategory::AccountSecurity => Ok(Self::AccountSecurity),
            email_pb::TransactionalEmailCategory::Billing => Ok(Self::Billing),
            email_pb::TransactionalEmailCategory::Reminder => Ok(Self::Reminder),
            email_pb::TransactionalEmailCategory::Unspecified => {
                Err(EmailCommandError::field("category"))
            }
        }
    }
}

impl TryFrom<email_pb::AccountSecurityEvent> for AccountSecurityEvent {
    type Error = EmailCommandError;

    fn try_from(value: email_pb::AccountSecurityEvent) -> Result<Self, Self::Error> {
        match value {
            email_pb::AccountSecurityEvent::EmailAdded => Ok(Self::EmailAdded),
            email_pb::AccountSecurityEvent::PrimaryEmailChanged => Ok(Self::PrimaryEmailChanged),
            email_pb::AccountSecurityEvent::AccountRecovered => Ok(Self::AccountRecovered),
            email_pb::AccountSecurityEvent::RecoveryReviewRequired => {
                Ok(Self::RecoveryReviewRequired)
            }
            email_pb::AccountSecurityEvent::Unspecified => {
                Err(EmailCommandError::field("template.event"))
            }
        }
    }
}

fn required_time(
    value: Option<prost_types::Timestamp>,
) -> Result<chrono::DateTime<chrono::Utc>, EmailCommandError> {
    datetime(
        value.ok_or_else(|| EmailCommandError::field("template.timestamp"))?,
        "template.timestamp",
    )
}
