use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;

use crate::operations_types::{band_value, label_kind_name, recommendation_value, review_state};

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
