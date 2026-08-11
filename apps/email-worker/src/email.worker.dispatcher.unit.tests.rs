use chrono::Utc;
use nvbes_email::{AccountSecurityEvent, EmailTemplate};

use super::{business_type, retry_policy, sealed};

#[test]
fn queue_retries_use_the_trigger_visibility_window() {
    let first = retry_policy("credential", 1);
    let last = retry_policy("credential", 4);
    assert_eq!(first.maximum_attempts, 4);
    assert_eq!(first.retry_delay.num_seconds(), 60);
    assert_eq!(last.retry_delay.num_seconds(), 60);
}

#[test]
fn test_capture_uses_the_existing_beta_business_type() {
    assert_eq!(
        business_type(&EmailTemplate::EmailVerificationV1 {
            user_name: "Ada".into(),
            verification_url: "https://account.nvbes.fr/verify-result?token=secret".into(),
            credential_expires_at: Utc::now(),
            timezone: "UTC".into(),
        }),
        "verification"
    );
}

#[test]
fn every_template_and_retry_category_has_a_stable_dispatch_policy() {
    let now = Utc::now();
    let templates = [
        (
            EmailTemplate::PasswordResetV1 {
                user_name: "Ada".into(),
                reset_url: "https://example.test/reset".into(),
                credential_expires_at: now,
            },
            "password_reset",
        ),
        (
            EmailTemplate::PasswordChangeCodeV1 {
                user_name: "Ada".into(),
                code: "123456".into(),
                credential_expires_at: now,
            },
            "password_change_code",
        ),
        (
            EmailTemplate::AccountSecurityV1 {
                event: AccountSecurityEvent::AccountRecovered,
                affected_email: None,
                previous_email: None,
                security_url: None,
            },
            "account_security",
        ),
        (
            EmailTemplate::BillingReceiptV1 {
                customer_name: None,
                amount_minor: 1200,
                currency: "EUR".into(),
                invoice_url: None,
                provider_name: None,
            },
            "billing_receipt",
        ),
        (
            EmailTemplate::BillingPaymentFailureV1 {
                customer_name: None,
                amount_minor: 1200,
                currency: "EUR".into(),
                billing_portal_url: None,
                invoice_url: None,
                provider_name: None,
            },
            "billing_payment_failure",
        ),
        (
            EmailTemplate::AccessReviewReminderV1 {
                reviewer_name: "Ada".into(),
                campaign_name: "Quarterly".into(),
                review_url: "https://example.test/review".into(),
                review_due_at: now,
            },
            "access_review_reminder",
        ),
    ];
    for (template, expected) in templates {
        assert_eq!(business_type(&template), expected);
    }
    for category in ["account_security", "billing", "reminder", "unknown"] {
        let policy = retry_policy(category, i32::MAX);
        assert!(policy.maximum_attempts >= 1);
        assert!(policy.retry_delay >= chrono::Duration::zero());
    }
    assert!(sealed(&[1], &[0; 11]).is_err());
    assert!(sealed(&[1], &[0; 12]).is_ok());
}
