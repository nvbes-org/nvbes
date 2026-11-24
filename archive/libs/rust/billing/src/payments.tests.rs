use super::{PaymentStatus, can_fallback_to_another_provider};

#[test]
fn fallback_allowed_only_for_retryable_statuses() {
    assert!(can_fallback_to_another_provider(PaymentStatus::Pending));
    assert!(can_fallback_to_another_provider(PaymentStatus::Failed));
    assert!(can_fallback_to_another_provider(PaymentStatus::Canceled));
    assert!(!can_fallback_to_another_provider(PaymentStatus::Captured));
    assert!(!can_fallback_to_another_provider(PaymentStatus::Authorized));
}
