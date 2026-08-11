use chrono::{DateTime, Duration, TimeZone, Utc};

use super::{
    AccountSecurityEvent, EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext,
    EmailTemplate,
};

pub(super) fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0)
        .single()
        .expect("valid timestamp")
}

pub(super) fn templates() -> Vec<EmailTemplate> {
    let expiry = now() + Duration::minutes(10);
    vec![
        EmailTemplate::EmailVerificationV1 {
            user_name: "Ada".into(),
            verification_url: "https://account.nvbes.eu/verify".into(),
            credential_expires_at: expiry,
            timezone: "Europe/Paris".into(),
        },
        EmailTemplate::PasswordResetV1 {
            user_name: "Ada".into(),
            reset_url: "https://account.nvbes.eu/reset".into(),
            credential_expires_at: expiry,
        },
        EmailTemplate::PasswordChangeCodeV1 {
            user_name: "Ada".into(),
            code: "123456".into(),
            credential_expires_at: expiry,
        },
        EmailTemplate::AccountSecurityV1 {
            event: AccountSecurityEvent::PrimaryEmailChanged,
            affected_email: Some("new@example.com".into()),
            previous_email: Some("old@example.com".into()),
            security_url: Some("https://account.nvbes.eu/security".into()),
        },
        EmailTemplate::BillingReceiptV1 {
            customer_name: Some("Ada".into()),
            amount_minor: 4_680,
            currency: "eur".into(),
            invoice_url: Some("https://billing.nvbes.eu/invoices/1".into()),
            provider_name: Some("Scaleway".into()),
        },
        EmailTemplate::BillingPaymentFailureV1 {
            customer_name: Some("Ada".into()),
            amount_minor: 4_680,
            currency: "eur".into(),
            billing_portal_url: Some("https://billing.nvbes.eu/portal".into()),
            invoice_url: Some("https://billing.nvbes.eu/invoices/1".into()),
            provider_name: Some("Scaleway".into()),
        },
        EmailTemplate::AccessReviewReminderV1 {
            reviewer_name: "Ada".into(),
            campaign_name: "Quarterly review".into(),
            review_url: "https://enterprise.nvbes.eu/reviews/1".into(),
            review_due_at: now() + Duration::days(2),
        },
    ]
}

pub(super) fn command(template: EmailTemplate) -> EmailCommand {
    let deliver_before = match &template {
        EmailTemplate::EmailVerificationV1 {
            credential_expires_at,
            ..
        }
        | EmailTemplate::PasswordResetV1 {
            credential_expires_at,
            ..
        }
        | EmailTemplate::PasswordChangeCodeV1 {
            credential_expires_at,
            ..
        } => *credential_expires_at,
        _ => now() + Duration::hours(1),
    };
    EmailCommand {
        context: EmailRequestContext {
            request_id: "request-1".into(),
            correlation_id: "correlation-1".into(),
            actor_principal_id: "principal-1".into(),
        },
        producer: "identity-service".into(),
        idempotency_key: EmailIdempotencyKey::new("email:test:1").expect("valid key"),
        recipient: EmailRecipient {
            email: "ada@example.com".into(),
            name: Some("Ada".into()),
        },
        category: template.category(),
        template,
        deliver_before,
    }
}
