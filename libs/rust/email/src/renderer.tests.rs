use chrono::{Duration, Utc};

use crate::{AccountSecurityEvent, EmailTemplate};

#[test]
fn every_template_renders_multipart_content() {
    let expiry = Utc::now() + Duration::minutes(10);
    let templates = vec![
        EmailTemplate::EmailVerificationV1 {
            user_name: "Ada".into(),
            verification_url: "https://account.nvbes.fr/verify?t=secret".into(),
            credential_expires_at: expiry,
        },
        EmailTemplate::PasswordResetV1 {
            user_name: "Ada".into(),
            reset_url: "https://account.nvbes.fr/reset?t=secret".into(),
            credential_expires_at: expiry,
        },
        EmailTemplate::PasswordChangeCodeV1 {
            user_name: "Ada".into(),
            code: "123456".into(),
            credential_expires_at: expiry,
        },
        EmailTemplate::AccountSecurityV1 {
            event: AccountSecurityEvent::AccountRecovered,
            affected_email: None,
            previous_email: None,
            security_url: Some("https://account.nvbes.fr/security".into()),
        },
        EmailTemplate::BillingReceiptV1 {
            customer_name: Some("Ada".into()),
            amount_minor: 4680,
            currency: "eur".into(),
            invoice_url: Some("https://billing.nvbes.fr/invoice/1".into()),
            provider_name: Some("Scaleway".into()),
        },
        EmailTemplate::BillingPaymentFailureV1 {
            customer_name: Some("Ada".into()),
            amount_minor: 4680,
            currency: "eur".into(),
            billing_portal_url: Some("https://billing.nvbes.fr".into()),
            invoice_url: None,
            provider_name: None,
        },
        EmailTemplate::AccessReviewReminderV1 {
            reviewer_name: "Ada".into(),
            campaign_name: "Quarterly review".into(),
            review_url: "https://enterprise.nvbes.fr/reviews/1".into(),
            review_due_at: expiry,
        },
    ];

    for template in templates {
        let rendered = template.render();
        assert!(!rendered.subject.is_empty());
        assert!(!rendered.text_body.is_empty());
        assert!(rendered.html_body.contains("<!doctype html>"));
        assert!(!rendered.preview_text.is_empty());
    }
}

#[test]
fn substitutions_are_escaped_in_html_but_preserved_in_text() {
    let rendered = EmailTemplate::AccountSecurityV1 {
        event: AccountSecurityEvent::EmailAdded,
        affected_email: Some("person+<script>@example.com".into()),
        previous_email: None,
        security_url: None,
    }
    .render();

    assert!(!rendered.html_body.contains("<script>"));
    assert!(rendered.html_body.contains("&lt;script&gt;"));
    assert!(rendered.text_body.contains("person+<script>@example.com"));
}
