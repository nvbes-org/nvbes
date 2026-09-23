use super::{
    CheckoutFraudPolicyContext, CheckoutFraudPolicyError, resolve_checkout_fraud_policy,
    validate_checkout_fraud_policy_overrides,
};
use crate::fraud::CheckoutFraudPolicy;

fn base_context() -> CheckoutFraudPolicyContext<'static> {
    CheckoutFraudPolicyContext {
        provider: Some("stripe"),
        plan_code: "enterprise",
        country: Some("DE"),
        amount_minor: 50_000,
    }
}

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
        base_context(),
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
            amount_minor: 25_000,
        },
    )
    .expect("policy should resolve");

    assert_eq!(policy.manual_review_threshold, 65);
}

#[test]
fn empty_or_blank_overrides_keep_base_policy() {
    let base = CheckoutFraudPolicy::default();
    for overrides in [None, Some(""), Some("   ")] {
        let policy = resolve_checkout_fraud_policy(base, overrides, base_context())
            .expect("blank overrides keep base");
        assert_eq!(policy, base);
        assert!(validate_checkout_fraud_policy_overrides(base, overrides).is_ok());
    }
}

#[test]
fn invalid_overrides_json_is_rejected() {
    let error =
        resolve_checkout_fraud_policy(CheckoutFraudPolicy::default(), Some("{"), base_context())
            .expect_err("invalid json");
    assert_eq!(error, CheckoutFraudPolicyError::InvalidJson);

    let error =
        validate_checkout_fraud_policy_overrides(CheckoutFraudPolicy::default(), Some("not-json"))
            .expect_err("invalid json");
    assert_eq!(error, CheckoutFraudPolicyError::InvalidJson);
}

#[test]
fn invalid_amount_range_is_rejected() {
    let error = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy::default(),
        Some(r#"[{"min_amount_minor":50000,"max_amount_minor":10000}]"#),
        base_context(),
    )
    .expect_err("min > max");
    assert_eq!(error, CheckoutFraudPolicyError::InvalidAmountRange);
}

#[test]
fn threshold_above_100_is_rejected_during_override_validation() {
    let error = validate_checkout_fraud_policy_overrides(
        CheckoutFraudPolicy::default(),
        Some(r#"[{"block_threshold":101}]"#),
    )
    .expect_err("threshold > 100");
    assert_eq!(error, CheckoutFraudPolicyError::InvalidThresholds);
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

#[test]
fn resolve_rejects_merged_thresholds_that_violate_ordering() {
    let error = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy {
            step_up_threshold: 60,
            manual_review_threshold: 75,
            block_threshold: 90,
        },
        Some(r#"[{"step_up_threshold":95}]"#),
        base_context(),
    )
    .expect_err("merged thresholds invalid");
    assert_eq!(error, CheckoutFraudPolicyError::InvalidThresholds);
}

#[test]
fn non_matching_overrides_leave_base_policy() {
    let base = CheckoutFraudPolicy::default();
    let policy = resolve_checkout_fraud_policy(
        base,
        Some(r#"[{"provider":"mollie","block_threshold":80}]"#),
        base_context(),
    )
    .expect("no match keeps base");
    assert_eq!(policy, base);
}

#[test]
fn equal_specificity_prefers_later_override() {
    let policy = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy::default(),
        Some(
            r#"[
              {"provider":"stripe","manual_review_threshold":70},
              {"provider":"STRIPE","manual_review_threshold":68}
            ]"#,
        ),
        base_context(),
    )
    .expect("later equal-specificity override wins");
    assert_eq!(policy.manual_review_threshold, 68);
}

#[test]
fn provider_country_and_plan_mismatches_exclude_override() {
    let base = CheckoutFraudPolicy::default();
    let cases = [
        r#"[{"provider":"stripe","plan_code":"starter","block_threshold":80}]"#,
        r#"[{"provider":"stripe","country":"FR","block_threshold":80}]"#,
        r#"[{"provider":"adyen","block_threshold":80}]"#,
        r#"[{"provider":"stripe","min_amount_minor":60000,"block_threshold":80}]"#,
        r#"[{"provider":"stripe","max_amount_minor":10000,"block_threshold":80}]"#,
    ];
    for overrides in cases {
        let policy = resolve_checkout_fraud_policy(base, Some(overrides), base_context())
            .expect("mismatched override ignored");
        assert_eq!(policy, base);
    }
}

#[test]
fn missing_context_fields_fail_required_string_matchers() {
    let base = CheckoutFraudPolicy::default();
    let policy = resolve_checkout_fraud_policy(
        base,
        Some(r#"[{"provider":"stripe","country":"DE","block_threshold":80}]"#),
        CheckoutFraudPolicyContext {
            provider: None,
            plan_code: "enterprise",
            country: None,
            amount_minor: 50_000,
        },
    )
    .expect("missing actuals do not match required expected");
    assert_eq!(policy, base);
}

#[test]
fn amount_bounds_inclusive_match_and_partial_threshold_override() {
    let policy = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy::default(),
        Some(
            r#"[{
              "min_amount_minor":50000,
              "max_amount_minor":50000,
              "step_up_threshold":55
            }]"#,
        ),
        base_context(),
    )
    .expect("inclusive amount bounds match");
    assert_eq!(policy.step_up_threshold, 55);
    assert_eq!(policy.manual_review_threshold, 75);
    assert_eq!(policy.block_threshold, 90);
}

#[test]
fn validate_accepts_well_formed_partial_overrides() {
    assert!(
        validate_checkout_fraud_policy_overrides(
            CheckoutFraudPolicy::default(),
            Some(r#"[{"provider":"stripe","manual_review_threshold":70}]"#),
        )
        .is_ok()
    );
}

#[test]
fn lower_specificity_override_does_not_replace_more_specific_match() {
    let policy = resolve_checkout_fraud_policy(
        CheckoutFraudPolicy::default(),
        Some(
            r#"[
              {"provider":"stripe","country":"FR","block_threshold":88},
              {"provider":"stripe","block_threshold":70}
            ]"#,
        ),
        CheckoutFraudPolicyContext {
            provider: Some("stripe"),
            plan_code: "team",
            country: Some("FR"),
            amount_minor: 10_000,
        },
    )
    .expect("more specific override wins");
    assert_eq!(policy.block_threshold, 88);
}
