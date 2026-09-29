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
            timezone: "Europe/Paris".into(),
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
        EmailTemplate::OperationalReadinessV1 {
            check_id: "deploy-123".into(),
            environment: "production".into(),
        },
    ];

    for template in templates {
        let rendered = template.render();
        assert!(!rendered.subject.is_empty());
        assert!(!rendered.text_body.is_empty());
        assert!(
            rendered
                .html_body
                .to_ascii_lowercase()
                .contains("<!doctype html")
        );
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

#[test]
fn verification_email_matches_account_brand_and_uses_a_readable_expiry() {
    let expiry = chrono::DateTime::parse_from_rfc3339("2026-08-04T10:37:53.510247251Z")
        .expect("test date should be valid")
        .with_timezone(&Utc);
    let rendered = EmailTemplate::EmailVerificationV1 {
        user_name: "Ada".into(),
        verification_url: "https://identity.nvbes.fr/verify?t=secret".into(),
        credential_expires_at: expiry,
        timezone: "Europe/Paris".into(),
    }
    .render();

    assert!(rendered.html_body.contains("nvbes <span"));
    assert!(rendered.html_body.contains("Account</span>"));
    assert!(rendered.html_body.contains("background-color:#247f7b"));
    assert!(rendered.html_body.contains("August 4, 2026 at 12:37"));
    assert!(rendered.text_body.contains("August 4, 2026 at 12:37"));
    assert!(rendered.html_body.contains("Europe/Paris (UTC+02:00)"));
    assert!(rendered.text_body.contains("Europe/Paris (UTC+02:00)"));
    assert!(rendered.html_body.contains("Privacy policy"));
    assert!(rendered.html_body.contains("Terms of service"));
    assert!(!rendered.html_body.contains("2026-08-04T10:37:53"));
    assert!(!rendered.text_body.contains("2026-08-04T10:37:53"));
    assert!(!rendered.html_body.contains("__NVBES_"));
}

#[test]
fn react_template_markers_inside_user_content_are_not_reinterpreted() {
    let rendered = EmailTemplate::EmailVerificationV1 {
        user_name: "__NVBES_MESSAGE__".into(),
        verification_url: "https://identity.nvbes.fr/verify".into(),
        credential_expires_at: Utc::now() + Duration::minutes(10),
        timezone: "UTC".into(),
    }
    .render();

    assert!(rendered.html_body.contains("Hi __NVBES_MESSAGE__,"));
    assert_eq!(
        rendered
            .html_body
            .matches("Verify your email address to activate your nvbes account.")
            .count(),
        1
    );
}

#[test]
fn renderer_helpers_preserve_optional_links_money_and_escaping() {
    assert_eq!(
        super::optional_link("Invoice", "View invoice", None),
        (String::new(), String::new())
    );
    let (text, html) =
        super::optional_link("Invoice", "View invoice", Some("https://example.com/i"));
    assert!(text.contains("https://example.com/i"));
    assert!(html.contains("href=\"https://example.com/i\""));
    assert!(html.contains("View invoice"));

    assert_eq!(super::money(4680, "eur"), "46.80 EUR");
    assert_eq!(super::money(5, "usd"), "0.05 USD");

    assert_eq!(super::optional_value(&None), "the configured address");
    assert_eq!(
        super::optional_value(&Some("ada@example.com".into())),
        "ada@example.com"
    );

    assert_eq!(super::escape(r#"a&b<"c">"#), "a&amp;b&lt;&quot;c&quot;&gt;");
    assert_eq!(super::escape("it's"), "it&#x27;s");
    assert_eq!(super::escape("plain"), "plain");
}
