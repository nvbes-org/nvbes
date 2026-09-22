use super::{PersonalGeoStoreError, UpsertPersonalGeoRangeInput, validate_personal_geo_input};
use crate::geo::types::GeoNetworkKind;

fn valid_input() -> UpsertPersonalGeoRangeInput {
    UpsertPersonalGeoRangeInput {
        network: "203.0.113.0/24".parse().unwrap(),
        country_code: "FR".to_string(),
        priority: 100,
        source_reference: Some("manual-test".to_string()),
        note: None,
        network_kind: GeoNetworkKind::Datacenter,
        risk_score: 70,
        risk_labels: vec!["manual".to_string(), "datacenter".to_string()],
        expires_at: None,
    }
}

#[test]
fn validates_country_code_and_priority_before_write() {
    assert!(validate_personal_geo_input(&valid_input()).is_ok());

    let mut invalid_country = valid_input();
    invalid_country.country_code = "XX".to_string();
    assert!(matches!(
        validate_personal_geo_input(&invalid_country),
        Err(PersonalGeoStoreError::Validation(_))
    ));

    let mut invalid_priority = valid_input();
    invalid_priority.priority = 0;
    assert!(matches!(
        validate_personal_geo_input(&invalid_priority),
        Err(PersonalGeoStoreError::InvalidPriority)
    ));
}

#[test]
fn rejects_risk_scores_above_one_hundred() {
    let mut input = valid_input();
    input.risk_score = 101;
    assert!(matches!(
        validate_personal_geo_input(&input),
        Err(PersonalGeoStoreError::InvalidRiskScore)
    ));
}
