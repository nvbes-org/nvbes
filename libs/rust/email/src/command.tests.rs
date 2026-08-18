use chrono::{Duration, TimeZone, Utc};

use super::{
    EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext,
    EmailTemplate,
};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0)
        .single()
        .expect("valid timestamp")
}

fn password_code_command(expiry: chrono::DateTime<Utc>) -> EmailCommand {
    EmailCommand {
        context: EmailRequestContext {
            request_id: "request-1".to_string(),
            correlation_id: "correlation-1".to_string(),
            actor_principal_id: "principal-1".to_string(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new("password-change:challenge-1")
            .expect("valid key"),
        recipient: EmailRecipient {
            email: "person@example.com".to_string(),
            name: Some("Ada".to_string()),
        },
        category: EmailCategory::Credential,
        template: EmailTemplate::PasswordChangeCodeV1 {
            user_name: "Ada".to_string(),
            code: "123456".to_string(),
            credential_expires_at: expiry,
        },
        deliver_before: expiry,
    }
}

#[test]
fn verification_code_deadline_must_equal_credential_expiry() {
    let now = now();
    let expiry = now + Duration::minutes(15);
    let mut command = password_code_command(expiry);
    command.deliver_before = expiry + Duration::seconds(1);

    let error = command
        .validate(now)
        .expect_err("deadline extension must fail");

    assert_eq!(error.field_name(), "template.credential_expires_at");
}

#[test]
fn verification_code_cannot_outlive_fifteen_minute_policy() {
    let now = now();
    let command = password_code_command(now + Duration::minutes(15) + Duration::seconds(1));

    let error = command
        .validate(now)
        .expect_err("overlong credential must fail");

    assert_eq!(error.field_name(), "template.credential_expires_at");
}

#[test]
fn valid_verification_code_preserves_deadline_on_wire() {
    let now = now();
    let expiry = now + Duration::minutes(10);
    let wire = password_code_command(expiry).into_proto();

    assert_eq!(
        wire.deliver_before.expect("deadline").seconds,
        expiry.timestamp()
    );
}

#[test]
fn category_must_match_closed_template_variant() {
    let now = now();
    let mut command = password_code_command(now + Duration::minutes(10));
    command.category = EmailCategory::Billing;

    let error = command
        .validate(now)
        .expect_err("category mismatch must fail");

    assert_eq!(error.field_name(), "category");
}

#[test]
fn recipient_and_idempotency_are_validated_before_transport() {
    assert!(EmailIdempotencyKey::new("contains spaces").is_err());

    let now = now();
    let mut command = password_code_command(now + Duration::minutes(10));
    command.recipient.email = "not-an-email".to_string();
    assert_eq!(
        command
            .validate(now)
            .expect_err("email must fail")
            .field_name(),
        "recipient.email"
    );
}
