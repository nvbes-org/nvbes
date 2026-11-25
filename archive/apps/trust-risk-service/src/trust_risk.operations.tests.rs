use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;

use crate::operations_types::{band_value, label_kind_name, recommendation_value, review_state};

use super::{MAX_RULE_SET_BYTES, operator_context_is_valid, rule_set_exceeds_budget};

#[test]
fn operations_conversions_do_not_invent_values() {
    assert_eq!(band_value("corrupt"), pb::RiskBand::Unspecified as i32);
    assert_eq!(
        recommendation_value("corrupt"),
        pb::RiskRecommendation::Unspecified as i32
    );
    assert_eq!(label_kind_name(99), None);
    assert!(review_state(pb::ReviewCaseState::Unspecified).is_err());
}

#[test]
fn rule_set_budget_and_operator_context_use_strict_boundaries() {
    assert_eq!(MAX_RULE_SET_BYTES, 262_144);
    assert!(!rule_set_exceeds_budget(MAX_RULE_SET_BYTES));
    assert!(rule_set_exceeds_budget(MAX_RULE_SET_BYTES + 1));
    assert!(!rule_set_exceeds_budget(10_000));

    let valid = pb::OperatorContext {
        caller: "abc".into(),
        actor: "abc".into(),
        reason: "why".into(),
        request_context: None,
    };
    assert!(operator_context_is_valid(&valid));

    let short_caller = pb::OperatorContext {
        caller: "ab".into(),
        ..valid.clone()
    };
    assert!(!operator_context_is_valid(&short_caller));

    let max_reason = pb::OperatorContext {
        reason: "r".repeat(300),
        ..valid.clone()
    };
    assert!(operator_context_is_valid(&max_reason));

    let overlong_reason = pb::OperatorContext {
        reason: "r".repeat(301),
        ..valid
    };
    assert!(!operator_context_is_valid(&overlong_reason));
}

#[cfg(feature = "database-tests")]
#[path = "trust_risk.operations.grpc.tests.rs"]
mod grpc;

#[cfg(feature = "database-tests")]
#[path = "trust_risk.operations.grpc.branch.tests.rs"]
mod grpc_branch;
