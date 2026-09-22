use chrono::Utc;
use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;
use uuid::Uuid;

use super::{
    EvaluationParams, band_value, evaluation, label_kind_name, map_label, map_review, map_rule,
    recommendation_value, review_case, review_case_row, review_state, review_state_name,
    rule_receipt, unavailable, uuid,
};
use crate::{
    labels_db::LabelPersistenceError,
    review_db::{ReviewCase, ReviewError, ReviewState},
    rules_db::{RuleOperationError, RuleSetReceipt},
};

#[test]
fn maps_band_and_recommendation_enums() {
    assert_eq!(band_value("low"), i32::from(pb::RiskBand::Low));
    assert_eq!(band_value("elevated"), i32::from(pb::RiskBand::Elevated));
    assert_eq!(band_value("high"), i32::from(pb::RiskBand::High));
    assert_eq!(band_value("critical"), i32::from(pb::RiskBand::Critical));
    assert_eq!(band_value("nope"), i32::from(pb::RiskBand::Unspecified));

    assert_eq!(
        recommendation_value("allow"),
        i32::from(pb::RiskRecommendation::Allow)
    );
    assert_eq!(
        recommendation_value("challenge"),
        i32::from(pb::RiskRecommendation::Challenge)
    );
    assert_eq!(
        recommendation_value("review"),
        i32::from(pb::RiskRecommendation::Review)
    );
    assert_eq!(
        recommendation_value("deny"),
        i32::from(pb::RiskRecommendation::Deny)
    );
    assert_eq!(
        recommendation_value("???"),
        i32::from(pb::RiskRecommendation::Unspecified)
    );
}

#[test]
fn maps_label_kinds() {
    assert_eq!(label_kind_name(1), Some("legitimate"));
    assert_eq!(label_kind_name(2), Some("confirmed_fraud"));
    assert_eq!(label_kind_name(3), Some("bot"));
    assert_eq!(label_kind_name(4), Some("account_takeover"));
    assert_eq!(label_kind_name(5), Some("chargeback"));
    assert_eq!(label_kind_name(6), Some("false_positive"));
    assert_eq!(label_kind_name(0), None);
    assert_eq!(label_kind_name(99), None);
}

#[test]
fn builds_evaluation_and_review_messages() {
    let now = Utc::now();
    let id = Uuid::new_v4();
    let eval = evaluation(EvaluationParams {
        id,
        score: 42,
        band: "elevated",
        recommendation: "challenge",
        reasons: vec!["velocity.burst".into()],
        feature_version: "features-v1",
        rule_version: "rules-v1",
        evaluated_at: now,
        expires_at: now,
    });
    assert_eq!(eval.evaluation_id, id.to_string());
    assert_eq!(eval.score, 42);
    assert_eq!(eval.reasons.len(), 1);
    assert!(!eval.duplicate);

    let case = review_case(ReviewCase {
        id: Uuid::new_v4(),
        evaluation_id: id,
        state: ReviewState::Open,
        assigned_to: Some("ada".into()),
        created_at: now,
        updated_at: now,
    });
    assert_eq!(case.state, i32::from(pb::ReviewCaseState::Open));
    assert_eq!(case.assigned_to.as_deref(), Some("ada"));

    let row = review_case_row((Uuid::new_v4(), id, "in_review".into(), None, now, now));
    assert_eq!(row.state, i32::from(pb::ReviewCaseState::InReview));

    let receipt = rule_receipt(RuleSetReceipt {
        version: "v2".into(),
        state: "active".into(),
        checksum: "abc".into(),
        updated_at: now,
    });
    assert_eq!(receipt.version, "v2");
    assert_eq!(receipt.checksum, "abc");
}

#[test]
fn review_state_conversions() {
    assert_eq!(
        review_state(pb::ReviewCaseState::Open).unwrap(),
        ReviewState::Open
    );
    assert_eq!(
        review_state(pb::ReviewCaseState::InReview).unwrap(),
        ReviewState::InReview
    );
    assert_eq!(
        review_state(pb::ReviewCaseState::Resolved).unwrap(),
        ReviewState::Resolved
    );
    assert_eq!(
        review_state(pb::ReviewCaseState::Inconclusive).unwrap(),
        ReviewState::Inconclusive
    );
    assert!(review_state(pb::ReviewCaseState::Unspecified).is_err());

    assert_eq!(review_state_name(pb::ReviewCaseState::Open), "open");
    assert_eq!(
        review_state_name(pb::ReviewCaseState::InReview),
        "in_review"
    );
    assert_eq!(review_state_name(pb::ReviewCaseState::Resolved), "resolved");
    assert_eq!(
        review_state_name(pb::ReviewCaseState::Inconclusive),
        "inconclusive"
    );
    assert_eq!(review_state_name(pb::ReviewCaseState::Unspecified), "");
}

#[test]
fn parses_uuid_and_maps_persistence_errors() {
    let id = Uuid::new_v4();
    assert_eq!(uuid(&id.to_string()).unwrap(), id);
    assert!(uuid("bad").is_err());

    assert_eq!(
        unavailable(sqlx::Error::Protocol("x".into())).code(),
        tonic::Code::Unavailable
    );
    assert_eq!(
        map_label(LabelPersistenceError::Conflict).code(),
        tonic::Code::AlreadyExists
    );
    assert_eq!(
        map_label(LabelPersistenceError::InvalidCorrection).code(),
        tonic::Code::FailedPrecondition
    );
    assert_eq!(
        map_review(ReviewError::NotFound).code(),
        tonic::Code::NotFound
    );
    assert_eq!(
        map_review(ReviewError::InvalidTransition).code(),
        tonic::Code::FailedPrecondition
    );
    assert_eq!(
        map_rule(RuleOperationError::Conflict).code(),
        tonic::Code::AlreadyExists
    );
    assert_eq!(
        map_rule(RuleOperationError::NotFound).code(),
        tonic::Code::NotFound
    );
    assert_eq!(
        map_rule(RuleOperationError::DualControl).code(),
        tonic::Code::FailedPrecondition
    );
    assert_eq!(
        map_rule(RuleOperationError::InvalidInput).code(),
        tonic::Code::InvalidArgument
    );
}
