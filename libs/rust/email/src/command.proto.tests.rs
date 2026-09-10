use chrono::Duration;

use crate::proto::nvbes::email::v1 as email_pb;

use super::test_support::{command, now, templates};
use super::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailReceipt, EmailTemplate, timestamp,
};

#[test]
fn every_template_round_trips_through_the_public_wire_contract() {
    for template in templates() {
        let original = command(template);
        let decoded =
            EmailCommand::try_from(original.clone().into_proto()).expect("wire round trip");
        assert_eq!(decoded, original);
    }
}

#[test]
fn receipt_and_timestamp_round_trip_preserve_all_fields() {
    let receipt = EmailReceipt {
        message_id: "message-1".into(),
        accepted_at: now(),
        deliver_before: now() + Duration::minutes(10),
        duplicate: true,
    };
    let wire = receipt.clone().into_proto();
    assert_eq!(wire.message_id, receipt.message_id);
    assert_eq!(wire.accepted_at, Some(timestamp(receipt.accepted_at)));
    assert_eq!(wire.deliver_before, Some(timestamp(receipt.deliver_before)));
    assert!(wire.duplicate);
}

#[test]
fn decoder_requires_every_envelope_component_and_valid_enums() {
    let base = command(templates().remove(0)).into_proto();
    let cases = [
        ("context", {
            let mut value = base.clone();
            value.context = None;
            value
        }),
        ("recipient", {
            let mut value = base.clone();
            value.recipient = None;
            value
        }),
        ("template", {
            let mut value = base.clone();
            value.template = None;
            value
        }),
        ("deliver_before", {
            let mut value = base.clone();
            value.deliver_before = None;
            value
        }),
        ("category", {
            let mut value = base.clone();
            value.category = 999;
            value
        }),
        ("category", {
            let mut value = base.clone();
            value.category = email_pb::TransactionalEmailCategory::Unspecified as i32;
            value
        }),
        ("idempotency_key", {
            let mut value = base.clone();
            value.idempotency_key = "unsafe key".into();
            value
        }),
    ];
    for (field, value) in cases {
        assert_eq!(
            EmailCommand::try_from(value).unwrap_err().field_name(),
            field
        );
    }

    let mut invalid_time = base;
    invalid_time.deliver_before = Some(prost_types::Timestamp {
        seconds: 0,
        nanos: -1,
    });
    assert_eq!(
        EmailCommand::try_from(invalid_time)
            .unwrap_err()
            .field_name(),
        "deliver_before"
    );
}

#[test]
fn decoder_defaults_verification_timezone_and_requires_template_timestamps() {
    let mut wire = command(templates().remove(0)).into_proto();
    let wrapper = wire.template.as_mut().expect("template");
    let email_pb::transactional_email_template::Template::EmailVerificationV1(value) =
        wrapper.template.as_mut().expect("variant")
    else {
        panic!("verification variant")
    };
    value.timezone = "  ".into();
    let decoded = EmailCommand::try_from(wire).expect("default timezone");
    let EmailTemplate::EmailVerificationV1 { timezone, .. } = decoded.template else {
        panic!("verification template")
    };
    assert_eq!(timezone, "UTC");

    for index in [0, 1, 2, 6] {
        let mut wire = command(templates().remove(index)).into_proto();
        let template = wire.template.as_mut().unwrap().template.as_mut().unwrap();
        match template {
            email_pb::transactional_email_template::Template::EmailVerificationV1(value) => {
                value.credential_expires_at = None
            }
            email_pb::transactional_email_template::Template::PasswordResetV1(value) => {
                value.credential_expires_at = None
            }
            email_pb::transactional_email_template::Template::PasswordChangeCodeV1(value) => {
                value.credential_expires_at = None
            }
            email_pb::transactional_email_template::Template::AccessReviewReminderV1(value) => {
                value.review_due_at = None
            }
            _ => unreachable!(),
        }
        assert_eq!(
            EmailCommand::try_from(wire).unwrap_err().field_name(),
            "template.timestamp"
        );
    }

    let empty = email_pb::TransactionalEmailTemplate { template: None };
    assert_eq!(
        EmailTemplate::try_from(empty).unwrap_err().field_name(),
        "template"
    );
}

#[test]
fn enum_decoders_cover_all_values_and_reject_unspecified_events() {
    let categories = [
        (
            email_pb::TransactionalEmailCategory::Credential,
            EmailCategory::Credential,
        ),
        (
            email_pb::TransactionalEmailCategory::AccountSecurity,
            EmailCategory::AccountSecurity,
        ),
        (
            email_pb::TransactionalEmailCategory::Billing,
            EmailCategory::Billing,
        ),
        (
            email_pb::TransactionalEmailCategory::Reminder,
            EmailCategory::Reminder,
        ),
        (
            email_pb::TransactionalEmailCategory::Operational,
            EmailCategory::Operational,
        ),
    ];
    for (wire, domain) in categories {
        assert_eq!(EmailCategory::try_from(wire).unwrap(), domain);
    }

    let events = [
        (
            email_pb::AccountSecurityEvent::MfaRecoveryCodesGenerated,
            AccountSecurityEvent::MfaRecoveryCodesGenerated,
        ),
        (
            email_pb::AccountSecurityEvent::MfaRecoveryStarted,
            AccountSecurityEvent::MfaRecoveryStarted,
        ),
        (
            email_pb::AccountSecurityEvent::MfaRecovered,
            AccountSecurityEvent::MfaRecovered,
        ),
        (
            email_pb::AccountSecurityEvent::MfaRecoveryCancelled,
            AccountSecurityEvent::MfaRecoveryCancelled,
        ),
        (
            email_pb::AccountSecurityEvent::EmailAdded,
            AccountSecurityEvent::EmailAdded,
        ),
        (
            email_pb::AccountSecurityEvent::PrimaryEmailChanged,
            AccountSecurityEvent::PrimaryEmailChanged,
        ),
        (
            email_pb::AccountSecurityEvent::AccountRecovered,
            AccountSecurityEvent::AccountRecovered,
        ),
        (
            email_pb::AccountSecurityEvent::RecoveryReviewRequired,
            AccountSecurityEvent::RecoveryReviewRequired,
        ),
    ];
    for (wire, domain) in events {
        assert_eq!(AccountSecurityEvent::try_from(wire).unwrap(), domain);
    }
    assert_eq!(
        AccountSecurityEvent::try_from(email_pb::AccountSecurityEvent::Unspecified)
            .unwrap_err()
            .field_name(),
        "template.event"
    );

    let mut wire = command(templates().remove(3)).into_proto();
    let wrapper = wire.template.as_mut().unwrap();
    let email_pb::transactional_email_template::Template::AccountSecurityV1(value) =
        wrapper.template.as_mut().unwrap()
    else {
        unreachable!()
    };
    value.event = 999;
    assert_eq!(
        EmailCommand::try_from(wire).unwrap_err().field_name(),
        "template.event"
    );
}
