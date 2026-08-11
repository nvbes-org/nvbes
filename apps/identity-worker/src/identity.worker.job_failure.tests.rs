use nvbes_email::EmailClientError;

use super::{
    JobExecutionError, JobFailureClass, MAX_CODE_CHARS, MAX_SUMMARY_CHARS, bounded_single_line,
};

#[test]
fn persisted_fields_are_single_line_bounded_and_unicode_safe() {
    let failure = JobExecutionError::permanent(
        &format!("bad\n{}", "é".repeat(MAX_CODE_CHARS * 2)),
        &format!("unsafe\r\n{}", "界".repeat(MAX_SUMMARY_CHARS * 2)),
    );

    assert!(!failure.code().contains('\n'));
    assert!(!failure.summary.contains('\n'));
    assert!(failure.code().chars().count() <= MAX_CODE_CHARS);
    assert!(failure.summary.chars().count() <= MAX_SUMMARY_CHARS);
    assert_eq!(bounded_single_line("  one\n\t two  ", 7), "one two");
    assert_eq!(bounded_single_line("ignored", 0), "");
}

#[test]
fn classes_drive_retryability_display_and_accessors() {
    let transient = JobExecutionError::transient("provider_down", "Provider unavailable");
    assert!(transient.is_retryable());
    assert_eq!(transient.class(), JobFailureClass::Transient);
    assert_eq!(transient.class().as_str(), "transient");
    assert_eq!(
        transient.to_string(),
        "transient:provider_down:Provider unavailable"
    );

    let permanent = JobExecutionError::permanent("invalid", "Invalid payload");
    assert!(!permanent.is_retryable());
    assert_eq!(permanent.class(), JobFailureClass::Permanent);
    assert_eq!(permanent.class().as_str(), "permanent");
}

#[test]
fn email_client_errors_are_classified_without_leaking_remote_details() {
    let cases = [
        (
            EmailClientError::Unavailable,
            true,
            "email_service_unavailable",
        ),
        (EmailClientError::Conflict, false, "email_command_conflict"),
        (
            EmailClientError::Unauthorized,
            false,
            "email_service_unauthorized",
        ),
        (EmailClientError::Protocol, false, "email_client_invalid"),
        (
            EmailClientError::Configuration("secret detail".to_string()),
            false,
            "email_client_invalid",
        ),
    ];

    for (source, retryable, code) in cases {
        let failure = JobExecutionError::from_email_client(&source);
        assert_eq!(failure.is_retryable(), retryable);
        assert_eq!(failure.code(), code);
        assert!(!failure.to_string().contains("secret detail"));
    }
}
