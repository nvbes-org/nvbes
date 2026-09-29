use super::{SafeLog, mask_email, mask_for_logging, mask_uuid};

#[test]
fn mask_for_logging_redacts_short_and_long_values() {
    assert_eq!(mask_for_logging("ab"), "[REDACTED]");
    assert_eq!(mask_for_logging("abc"), "[REDACTED]");
    assert_eq!(mask_for_logging("abcd"), "a**d");
    assert_eq!(mask_for_logging("abcdefghijklm"), "a**********m");
}

#[test]
fn mask_email_keeps_domain_and_masks_local_part() {
    assert_eq!(mask_email("ab@example.com"), "***@example.com");
    assert_eq!(mask_email("alice@example.com"), "al***@example.com");
    assert_eq!(mask_email("not-an-email"), "[INVALID_EMAIL]");
}

#[test]
fn mask_uuid_keeps_prefix_when_long_enough() {
    assert_eq!(
        mask_uuid("01234567-89ab-cdef-0123-456789abcdef"),
        "01234567-***"
    );
    assert_eq!(mask_uuid("short"), "[REDACTED]");
}

#[test]
fn safe_log_debug_masks_string_and_option_values() {
    assert_eq!(
        format!("{:?}", SafeLog("secret-value".to_string())),
        "s**********e"
    );
    assert_eq!(format!("{:?}", SafeLog(Some("abcd".to_string()))), "a**d");
    assert_eq!(format!("{:?}", SafeLog::<Option<String>>(None)), "None");
}
