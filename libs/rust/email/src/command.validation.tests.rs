use chrono::Duration;

use super::test_support::{command, now, templates};
use super::{EmailIdempotencyKey, EmailTemplate, normalized_timezone};

fn assert_invalid(mutator: impl FnOnce(&mut super::EmailCommand), field: &'static str) {
    let mut value = command(templates().remove(2));
    mutator(&mut value);
    assert_eq!(
        value
            .validate(now())
            .expect_err("command must fail")
            .field_name(),
        field
    );
}

#[test]
fn every_closed_template_variant_is_valid_and_self_describing() {
    let expected = [
        ("email_verification", "credential"),
        ("password_reset", "credential"),
        ("password_change_code", "credential"),
        ("account_security", "account_security"),
        ("billing_receipt", "billing"),
        ("billing_payment_failure", "billing"),
        ("access_review_reminder", "reminder"),
        ("operational_readiness", "operational"),
    ];

    for (template, (name, category)) in templates().into_iter().zip(expected) {
        assert_eq!(template.name_and_version(), (name, 1));
        assert_eq!(template.category().as_str(), category);
        command(template).validate(now()).expect("valid command");
    }
}

#[test]
fn timezone_normalization_is_canonical_and_falls_back_to_utc() {
    assert_eq!(normalized_timezone(" Europe/Paris "), "Europe/Paris");
    assert_eq!(normalized_timezone("not/a-zone"), "UTC");
}

#[test]
fn identifiers_reject_empty_long_or_unsafe_values() {
    for value in ["", "contains spaces", "contains/slash"] {
        assert_eq!(
            EmailIdempotencyKey::new(value)
                .expect_err("unsafe key")
                .field_name(),
            "idempotency_key"
        );
    }
    assert!(EmailIdempotencyKey::new("a".repeat(200)).is_ok());
    assert!(EmailIdempotencyKey::new("a".repeat(201)).is_err());

    assert_invalid(|value| value.producer.clear(), "producer");
    assert_invalid(|value| value.producer = "a".repeat(65), "producer");
    assert_invalid(
        |value| value.producer = "unsafe producer".into(),
        "producer",
    );
}

#[test]
fn envelope_fields_enforce_address_name_and_future_deadline() {
    assert_invalid(
        |value| value.recipient.email = "not-an-email".into(),
        "recipient.email",
    );
    for name in [" ".to_string(), "line\nbreak".to_string(), "a".repeat(201)] {
        assert_invalid(|value| value.recipient.name = Some(name), "recipient.name");
    }
    let mut unnamed = command(templates().remove(3));
    unnamed.recipient.name = None;
    unnamed.validate(now()).expect("recipient name is optional");
    assert_invalid(|value| value.deliver_before = now(), "deliver_before");
}

#[test]
fn persisted_email_fields_have_explicit_size_limits() {
    assert_invalid(
        |value| value.recipient.email = format!("{}@example.com", "a".repeat(321)),
        "recipient.email",
    );

    let mut verification = command(templates().remove(0));
    if let EmailTemplate::EmailVerificationV1 {
        verification_url, ..
    } = &mut verification.template
    {
        *verification_url = format!("https://example.com/{}", "a".repeat(4096));
    }
    assert_eq!(
        verification.validate(now()).unwrap_err().field_name(),
        "template.url"
    );

    let mut security = command(templates().remove(3));
    if let EmailTemplate::AccountSecurityV1 { affected_email, .. } = &mut security.template {
        *affected_email = Some(format!("{}@example.com", "a".repeat(321)));
    }
    assert_eq!(
        security.validate(now()).unwrap_err().field_name(),
        "template.affected_email"
    );

    let mut receipt = command(templates().remove(4));
    if let EmailTemplate::BillingReceiptV1 {
        customer_name,
        provider_name,
        ..
    } = &mut receipt.template
    {
        *customer_name = Some("a".repeat(201));
        *provider_name = Some("provider".into());
    }
    assert_eq!(
        receipt.validate(now()).unwrap_err().field_name(),
        "template.customer_name"
    );
}

#[test]
fn credential_templates_enforce_content_urls_expiry_and_timezone() {
    let mut verification = command(templates().remove(0));
    if let EmailTemplate::EmailVerificationV1 { timezone, .. } = &mut verification.template {
        *timezone = "Mars/Olympus".into();
    }
    assert_eq!(
        verification.validate(now()).unwrap_err().field_name(),
        "template.timezone"
    );

    for index in 0..3 {
        let mut value = command(templates().remove(index));
        match &mut value.template {
            EmailTemplate::EmailVerificationV1 { user_name, .. }
            | EmailTemplate::PasswordResetV1 { user_name, .. }
            | EmailTemplate::PasswordChangeCodeV1 { user_name, .. } => user_name.clear(),
            _ => unreachable!(),
        }
        assert_eq!(
            value.validate(now()).unwrap_err().field_name(),
            "template.user_name"
        );
    }

    for index in 0..2 {
        let mut value = command(templates().remove(index));
        match &mut value.template {
            EmailTemplate::EmailVerificationV1 {
                verification_url, ..
            } => *verification_url = "http://example.com/token".into(),
            EmailTemplate::PasswordResetV1 { reset_url, .. } => *reset_url = "relative".into(),
            _ => unreachable!(),
        }
        assert_eq!(
            value.validate(now()).unwrap_err().field_name(),
            "template.url"
        );
    }

    let mut local = command(templates().remove(1));
    if let EmailTemplate::PasswordResetV1 { reset_url, .. } = &mut local.template {
        *reset_url = "http://127.0.0.1/reset".into();
    }
    local
        .validate(now())
        .expect("loopback HTTP is safe for tests");

    let mut expired = command(templates().remove(1));
    expired.deliver_before = now() - Duration::seconds(1);
    assert_eq!(
        expired.validate(now()).unwrap_err().field_name(),
        "deliver_before"
    );

    let mut long_reset = command(templates().remove(1));
    let expiry = now() + Duration::hours(2) + Duration::seconds(1);
    long_reset.deliver_before = expiry;
    if let EmailTemplate::PasswordResetV1 {
        credential_expires_at,
        ..
    } = &mut long_reset.template
    {
        *credential_expires_at = expiry;
    }
    assert_eq!(
        long_reset.validate(now()).unwrap_err().field_name(),
        "template.credential_expires_at"
    );
}

#[test]
fn code_money_optional_urls_and_reminders_reject_invalid_values() {
    for code in ["12345", "12345a"] {
        let mut value = command(templates().remove(2));
        if let EmailTemplate::PasswordChangeCodeV1 { code: current, .. } = &mut value.template {
            *current = code.into();
        }
        assert_eq!(
            value.validate(now()).unwrap_err().field_name(),
            "template.code"
        );
    }

    for index in [4, 5] {
        let mut value = command(templates().remove(index));
        match &mut value.template {
            EmailTemplate::BillingReceiptV1 { amount_minor, .. }
            | EmailTemplate::BillingPaymentFailureV1 { amount_minor, .. } => *amount_minor = -1,
            _ => unreachable!(),
        }
        assert_eq!(
            value.validate(now()).unwrap_err().field_name(),
            "template.money"
        );
    }
    for currency in ["EU", "€€€"] {
        let mut value = command(templates().remove(4));
        if let EmailTemplate::BillingReceiptV1 {
            currency: current, ..
        } = &mut value.template
        {
            *current = currency.into();
        }
        assert_eq!(
            value.validate(now()).unwrap_err().field_name(),
            "template.money"
        );
    }

    let mut receipt = command(templates().remove(4));
    if let EmailTemplate::BillingReceiptV1 { invoice_url, .. } = &mut receipt.template {
        *invoice_url = Some("http://billing.example.com".into());
    }
    assert_eq!(
        receipt.validate(now()).unwrap_err().field_name(),
        "template.invoice_url"
    );

    let mut failure = command(templates().remove(5));
    if let EmailTemplate::BillingPaymentFailureV1 {
        billing_portal_url, ..
    } = &mut failure.template
    {
        *billing_portal_url = Some("ftp://billing.example.com".into());
    }
    assert_eq!(
        failure.validate(now()).unwrap_err().field_name(),
        "template.billing_portal_url"
    );

    for field in ["reviewer", "campaign"] {
        let mut reminder = command(templates().remove(6));
        if let EmailTemplate::AccessReviewReminderV1 {
            reviewer_name,
            campaign_name,
            ..
        } = &mut reminder.template
        {
            if field == "reviewer" {
                reviewer_name.clear()
            } else {
                campaign_name.clear()
            }
        }
        assert_eq!(
            reminder.validate(now()).unwrap_err().field_name(),
            if field == "reviewer" {
                "template.reviewer_name"
            } else {
                "template.campaign_name"
            }
        );
    }
    let mut reminder = command(templates().remove(6));
    reminder.deliver_before = now() + Duration::hours(24) + Duration::seconds(1);
    assert_eq!(
        reminder.validate(now()).unwrap_err().field_name(),
        "deliver_before"
    );
}

#[test]
fn operational_readiness_is_bounded_and_uses_safe_identifiers() {
    let mut unsafe_check = command(templates().remove(7));
    if let EmailTemplate::OperationalReadinessV1 { check_id, .. } = &mut unsafe_check.template {
        *check_id = "unsafe check".into();
    }
    assert_eq!(
        unsafe_check.validate(now()).unwrap_err().field_name(),
        "template.check_id"
    );

    let mut long_lived = command(templates().remove(7));
    long_lived.deliver_before = now() + Duration::minutes(30) + Duration::seconds(1);
    assert_eq!(
        long_lived.validate(now()).unwrap_err().field_name(),
        "deliver_before"
    );
}
