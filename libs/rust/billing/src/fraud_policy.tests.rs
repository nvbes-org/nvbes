use super::{
    CheckoutFraudPolicyContext, CheckoutFraudPolicyError, resolve_checkout_fraud_policy,
    validate_checkout_fraud_policy_overrides,
};
use crate::fraud::CheckoutFraudPolicy;

#[test]
fn most_specific_matching_override_wins() {
    let policy = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy::default(),
        Some(
            r#"[
              {"provider":"stripe","manual_review_threshold":70},
              {"provider":"stripe","plan_code":"enterprise","country":"DE","block_threshold":80}
            ]"#,
        ),
        CheckoutFraudPolicyContext {
            provider: Some("stripe"),
            plan_code: "enterprise",
            country: Some("DE"),
            amount_minor: 50000,
        },
    )
    .expect("policy should resolve");

    assert_eq!(policy.manual_review_threshold, 75);
    assert_eq!(policy.block_threshold, 80);
}

#[test]
fn amount_range_override_matches_checkout_amount() {
    let policy = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy::default(),
        Some(r#"[{"min_amount_minor":20000,"manual_review_threshold":65}]"#),
        CheckoutFraudPolicyContext {
            provider: Some("mollie"),
            plan_code: "team",
            country: Some("FR"),
            amount_minor: 25000,
        },
    )
    .expect("policy should resolve");

    assert_eq!(policy.manual_review_threshold, 65);
}

#[test]
fn invalid_effective_thresholds_are_rejected() {
    let error = validate_checkout_fraud_policy_overrides(
        CheckoutFraudPolicy::default(),
        Some(r#"[{"step_up_threshold":95,"manual_review_threshold":75}]"#),
    )
    .expect_err("override should be invalid");

    assert_eq!(error, CheckoutFraudPolicyError::InvalidThresholds);
}
