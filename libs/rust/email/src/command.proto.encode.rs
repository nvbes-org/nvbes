use crate::proto::nvbes::{email::v1 as email_pb, platform::v1 as platform_pb};

use super::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailReceipt, EmailTemplate, timestamp,
};

impl EmailCommand {
    pub fn into_proto(self) -> email_pb::SubmitEmailRequest {
        email_pb::SubmitEmailRequest {
            context: Some(platform_pb::RequestContext {
                request_id: self.context.request_id,
                correlation_id: self.context.correlation_id,
                actor_principal_id: self.context.actor_principal_id,
                tenant: None,
            }),
            producer: self.producer,
            idempotency_key: self.idempotency_key.0,
            recipient: Some(email_pb::EmailRecipient {
                email: self.recipient.email,
                name: self.recipient.name,
            }),
            category: self.category.as_proto() as i32,
            template: Some(self.template.into_proto()),
            deliver_before: Some(timestamp(self.deliver_before)),
        }
    }
}

impl EmailReceipt {
    pub fn into_proto(self) -> email_pb::EmailReceipt {
        email_pb::EmailReceipt {
            message_id: self.message_id,
            accepted_at: Some(timestamp(self.accepted_at)),
            deliver_before: Some(timestamp(self.deliver_before)),
            duplicate: self.duplicate,
        }
    }
}

impl EmailTemplate {
    fn into_proto(self) -> email_pb::TransactionalEmailTemplate {
        use email_pb::transactional_email_template::Template;
        let template = match self {
            Self::EmailVerificationV1 {
                user_name,
                verification_url,
                credential_expires_at,
                timezone,
            } => Template::EmailVerificationV1(email_pb::EmailVerificationV1 {
                user_name,
                verification_url,
                credential_expires_at: Some(timestamp(credential_expires_at)),
                timezone,
            }),
            Self::PasswordResetV1 {
                user_name,
                reset_url,
                credential_expires_at,
            } => Template::PasswordResetV1(email_pb::PasswordResetV1 {
                user_name,
                reset_url,
                credential_expires_at: Some(timestamp(credential_expires_at)),
            }),
            Self::PasswordChangeCodeV1 {
                user_name,
                code,
                credential_expires_at,
            } => Template::PasswordChangeCodeV1(email_pb::PasswordChangeCodeV1 {
                user_name,
                code,
                credential_expires_at: Some(timestamp(credential_expires_at)),
            }),
            Self::AccountSecurityV1 {
                event,
                affected_email,
                previous_email,
                security_url,
            } => Template::AccountSecurityV1(email_pb::AccountSecurityV1 {
                event: event.as_proto() as i32,
                affected_email,
                previous_email,
                security_url,
            }),
            Self::BillingReceiptV1 {
                customer_name,
                amount_minor,
                currency,
                invoice_url,
                provider_name,
            } => Template::BillingReceiptV1(email_pb::BillingReceiptV1 {
                customer_name,
                amount_minor,
                currency,
                invoice_url,
                provider_name,
            }),
            Self::BillingPaymentFailureV1 {
                customer_name,
                amount_minor,
                currency,
                billing_portal_url,
                invoice_url,
                provider_name,
            } => Template::BillingPaymentFailureV1(email_pb::BillingPaymentFailureV1 {
                customer_name,
                amount_minor,
                currency,
                billing_portal_url,
                invoice_url,
                provider_name,
            }),
            Self::AccessReviewReminderV1 {
                reviewer_name,
                campaign_name,
                review_url,
                review_due_at,
            } => Template::AccessReviewReminderV1(email_pb::AccessReviewReminderV1 {
                reviewer_name,
                campaign_name,
                review_url,
                review_due_at: Some(timestamp(review_due_at)),
            }),
            Self::OperationalReadinessV1 {
                check_id,
                environment,
            } => Template::OperationalReadinessV1(email_pb::OperationalReadinessV1 {
                check_id,
                environment,
            }),
        };
        email_pb::TransactionalEmailTemplate {
            template: Some(template),
        }
    }
}

impl EmailCategory {
    fn as_proto(self) -> email_pb::TransactionalEmailCategory {
        match self {
            Self::Credential => email_pb::TransactionalEmailCategory::Credential,
            Self::AccountSecurity => email_pb::TransactionalEmailCategory::AccountSecurity,
            Self::Billing => email_pb::TransactionalEmailCategory::Billing,
            Self::Reminder => email_pb::TransactionalEmailCategory::Reminder,
            Self::Operational => email_pb::TransactionalEmailCategory::Operational,
        }
    }
}

impl AccountSecurityEvent {
    fn as_proto(self) -> email_pb::AccountSecurityEvent {
        match self {
            Self::EmailAdded => email_pb::AccountSecurityEvent::EmailAdded,
            Self::PrimaryEmailChanged => email_pb::AccountSecurityEvent::PrimaryEmailChanged,
            Self::AccountRecovered => email_pb::AccountSecurityEvent::AccountRecovered,
            Self::RecoveryReviewRequired => email_pb::AccountSecurityEvent::RecoveryReviewRequired,
            Self::MfaRecoveryCodesGenerated => {
                email_pb::AccountSecurityEvent::MfaRecoveryCodesGenerated
            }
            Self::MfaRecoveryStarted => email_pb::AccountSecurityEvent::MfaRecoveryStarted,
            Self::MfaRecovered => email_pb::AccountSecurityEvent::MfaRecovered,
            Self::MfaRecoveryCancelled => email_pb::AccountSecurityEvent::MfaRecoveryCancelled,
            Self::PasswordRecovered => email_pb::AccountSecurityEvent::PasswordRecovered,
        }
    }
}
