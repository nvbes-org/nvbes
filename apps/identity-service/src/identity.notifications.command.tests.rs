use super::*;

#[test]
fn recovery_events_round_trip_with_distinct_messages_and_stable_retry_identity() {
    let occurred = Utc::now();
    let principal = Uuid::new_v4();
    let cases = [
        (
            "identity.mfa_recovery_codes_generated",
            "Previous recovery codes can no longer be used.",
        ),
        ("identity.mfa_recovery_started", "not yet complete"),
        ("identity.mfa_recovered", "Your password was not changed"),
        (
            "identity.mfa_recovery_cancelled",
            "revoked sessions remain closed",
        ),
    ];
    let mut subjects = std::collections::HashSet::new();
    for (event, statement) in cases {
        let id = Uuid::new_v4();
        let first = recovery_command(
            id,
            principal,
            event,
            occurred,
            "owner@example.invalid".into(),
            occurred,
        )
        .unwrap();
        let retry = recovery_command(
            id,
            principal,
            event,
            occurred,
            "owner@example.invalid".into(),
            occurred + Duration::hours(1),
        )
        .unwrap();
        assert_eq!(first, retry);
        assert_eq!(first.category, EmailCategory::AccountSecurity);
        assert_eq!(first.deliver_before, occurred + Duration::hours(24));
        let decoded = EmailCommand::try_from(first.clone().into_proto()).unwrap();
        assert_eq!(decoded, first);
        let rendered = decoded.template.render();
        assert!(subjects.insert(rendered.subject));
        assert!(rendered.text_body.contains(statement));
        assert!(rendered.html_body.contains(statement));
        assert!(!rendered.text_body.contains("token="));
        assert!(!rendered.text_body.contains("nvr1_"));
        assert!(!rendered.text_body.contains("owner@example.invalid"));
        assert!(!rendered.text_body.contains("Your password was reset"));
    }
}

#[test]
fn unsupported_expired_future_and_invalid_recipient_notifications_are_refused() {
    let now = Utc::now();
    for (event, occurred, recipient) in [
        ("identity.arbitrary", now, "owner@example.invalid"),
        (
            "identity.mfa_recovered",
            now - Duration::hours(24),
            "owner@example.invalid",
        ),
        (
            "identity.mfa_recovered",
            now + Duration::seconds(1),
            "owner@example.invalid",
        ),
        ("identity.mfa_recovered", now, "invalid\nrecipient"),
    ] {
        assert!(
            recovery_command(
                Uuid::new_v4(),
                Uuid::new_v4(),
                event,
                occurred,
                recipient.into(),
                now
            )
            .is_err()
        );
    }
}
