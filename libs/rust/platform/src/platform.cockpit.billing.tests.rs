use super::*;

#[test]
fn strictly_rejects_live_stripe_credentials() {
    assert_eq!(
        BillingCockpitView::assert_test_mode("sk_live_12345"),
        Err(BillingCockpitError::LiveModeForbidden)
    );
    assert_eq!(
        BillingCockpitView::assert_test_mode("whsec_live_9999"),
        Err(BillingCockpitError::LiveModeForbidden)
    );
    assert!(BillingCockpitView::assert_test_mode("sk_test_12345").is_ok());
}

#[test]
fn validates_manual_reconciliation_reason() {
    assert!(
        BillingCockpitView::validate_manual_reconciliation("Manual bank transfer verified").is_ok()
    );
    assert_eq!(
        BillingCockpitView::validate_manual_reconciliation("no"),
        Err(BillingCockpitError::InvalidReconciliationReason)
    );
}

#[test]
fn builds_reconciliation_summary_in_test_mode() {
    let summary = BillingCockpitView::build_summary(10, 1, 2, 3, Some(Utc::now()));
    assert!(summary.test_mode);
    assert_eq!(summary.webhook_events_24h, 10);
    assert_eq!(summary.unverified_webhooks_count, 1);
    assert_eq!(summary.pending_reconciliations_count, 2);
    assert_eq!(summary.reconciliation_mismatch_count, 3);
    assert!(summary.last_reconciliation_at.is_some());
}
