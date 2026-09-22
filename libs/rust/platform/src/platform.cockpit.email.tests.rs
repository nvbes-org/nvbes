use super::*;

#[test]
fn masks_email_addresses_for_privacy() {
    assert_eq!(
        EmailCockpitView::mask_email("user@example.com"),
        "u***r@example.com"
    );
    assert_eq!(
        EmailCockpitView::mask_email("ab@domain.com"),
        "*@domain.com"
    );
    assert_eq!(EmailCockpitView::mask_email("invalid"), "redacted");
}

#[test]
fn validates_suppression_reasons() {
    assert!(EmailCockpitView::validate_suppression_request("Spam trap hit").is_ok());
    assert_eq!(
        EmailCockpitView::validate_suppression_request("  "),
        Err(EmailCockpitError::InvalidSuppressionReason)
    );
}

#[test]
fn builds_operations_summary_counts() {
    let summary = EmailCockpitView::build_summary(1, 2, 3, 4, 5, 6, 7);
    assert_eq!(summary.queued_count, 1);
    assert_eq!(summary.sent_count_24h, 2);
    assert_eq!(summary.delivered_count_24h, 3);
    assert_eq!(summary.failed_count_24h, 4);
    assert_eq!(summary.bounce_count_24h, 5);
    assert_eq!(summary.active_suppressions_count, 6);
    assert_eq!(summary.unprocessed_events_count, 7);
}
