use std::collections::BTreeMap;

use crate::proto::nvbes::trust_risk::v1::{DataScope, SubjectKind};
use crate::{
    rules::{FeatureMap, RuleSet, evaluate},
    signal::{SignalError, SubjectReference},
};

fn wire_subject(opaque_id: &str) -> crate::proto::nvbes::trust_risk::v1::SubjectReference {
    crate::proto::nvbes::trust_risk::v1::SubjectReference {
        kind: SubjectKind::Principal.into(),
        namespace: "nvbes.identity".to_string(),
        opaque_id: opaque_id.to_string(),
        scope: DataScope::Regional.into(),
        tenant_id: Some("018f7f2d-fc7d-7b7a-9f72-3abddda8d001".to_string()),
    }
}

#[test]
fn rejects_email_like_subject_identifiers() {
    let error = SubjectReference::try_from(wire_subject("ada@example.com")).unwrap_err();
    assert_eq!(error, SignalError::ForbiddenSubject);
}

#[test]
fn accepts_opaque_subject_identifiers() {
    let subject =
        SubjectReference::try_from(wire_subject("018f7f2d-fc7d-7b7a-9f72-3abddda8d001")).unwrap();
    assert_eq!(subject.namespace(), "nvbes.identity");
}

#[test]
fn evaluator_is_deterministic_bounded_and_orders_reasons() {
    let rules = RuleSet::from_json(
        br#"{
          "version":"baseline-v1",
          "feature_version":"features-v1",
          "thresholds":{"challenge":40,"review":70,"deny":90},
          "rules":[
            {
              "code":"automation-high",
              "predicate":{"op":"gte","feature":"automation_confidence_max_1h","value":0.8},
              "score_delta":75,
              "reasons":["automation_high"],
              "minimum_recommendation":"challenge"
            },
            {
              "code":"network-high",
              "predicate":{"op":"gte","feature":"network_risk_score_max","value":80},
              "score_delta":50,
              "reasons":["network_high"],
              "minimum_recommendation":null
            }
          ]
        }"#,
    )
    .unwrap();
    let features: FeatureMap = BTreeMap::from([
        ("automation_confidence_max_1h".to_string(), 0.92),
        ("network_risk_score_max".to_string(), 95.0),
    ]);

    let first = evaluate(&rules, &features);
    let second = evaluate(&rules, &features);

    assert_eq!(first, second);
    assert_eq!(first.score, 100);
    assert_eq!(first.reason_codes, vec!["automation_high", "network_high"]);
}

#[test]
fn rule_validation_rejects_excessive_predicate_depth() {
    let mut predicate = serde_json::json!({
        "op": "gte",
        "feature": "events_1h",
        "value": 10
    });
    for _ in 0..9 {
        predicate = serde_json::json!({"op":"not", "predicate": predicate});
    }
    let document = serde_json::json!({
        "version":"deep-v1",
        "feature_version":"features-v1",
        "thresholds":{"challenge":40,"review":70,"deny":90},
        "rules":[{
            "code":"too-deep",
            "predicate":predicate,
            "score_delta":1,
            "reasons":["too_deep"],
            "minimum_recommendation":null
        }]
    });

    assert!(RuleSet::from_json(&serde_json::to_vec(&document).unwrap()).is_err());
}
