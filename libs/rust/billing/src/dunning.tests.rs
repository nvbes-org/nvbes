use super::{AccessPolicyState, policy_after_payment_failure, policy_after_payment_success};

#[test]
fn dunning_policy_escalates_with_failures() {
    assert_eq!(policy_after_payment_failure(0), AccessPolicyState::Warning);
    assert_eq!(policy_after_payment_failure(2), AccessPolicyState::Grace);
    assert_eq!(policy_after_payment_failure(4), AccessPolicyState::Degraded);
    assert_eq!(
        policy_after_payment_failure(5),
        AccessPolicyState::Suspended
    );
    assert_eq!(policy_after_payment_success(), AccessPolicyState::Active);
}
