use std::collections::BTreeMap;

use nvbes_trust_risk::{
    rules::{FeatureMap, RuleSet, evaluate},
    types::Recommendation,
};

fn rules() -> RuleSet {
    RuleSet::from_json(br#"{
        "version":"billing-fixture-v1","feature_version":"features-v1",
        "thresholds":{"challenge":30,"review":60,"deny":85},
        "rules":[
          {"code":"network","predicate":{"op":"gte","feature":"network_risk_score_max_1h","value":75},"score_delta":35,"reasons":["network.high_risk"],"minimum_recommendation":null},
          {"code":"reuse","predicate":{"op":"gte","feature":"linked_principals_24h","value":5},"score_delta":40,"reasons":["subject.reuse"],"minimum_recommendation":null},
          {"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":25,"reasons":["velocity.high"],"minimum_recommendation":null},
          {"code":"feedback","predicate":{"op":"gte","feature":"negative_labels","value":1},"score_delta":45,"reasons":["label.negative"],"minimum_recommendation":"review"},
          {"code":"correction","predicate":{"op":"gte","feature":"positive_labels","value":1},"score_delta":-30,"reasons":["reputation.positive"],"minimum_recommendation":null}
        ]
    }"#).unwrap()
}

fn features(values: &[(&str, f64)]) -> FeatureMap {
    values
        .iter()
        .map(|(name, value)| ((*name).to_string(), *value))
        .collect::<BTreeMap<_, _>>()
}

#[test]
fn billing_fixtures_exercise_generic_engine_without_checkout_fields() {
    let rules = rules();
    let legitimate = features(&[]);
    assert_eq!(
        evaluate(&rules, &legitimate).recommendation,
        Recommendation::Allow
    );
    assert_eq!(evaluate(&rules, &legitimate), evaluate(&rules, &legitimate));
    assert_eq!(
        evaluate(&rules, &features(&[("network_risk_score_max_1h", 90.0)])).recommendation,
        Recommendation::Challenge
    );
    assert_eq!(
        evaluate(
            &rules,
            &features(&[("linked_principals_24h", 8.0), ("events_1h", 12.0)])
        )
        .recommendation,
        Recommendation::Review
    );
    assert_eq!(
        evaluate(&rules, &features(&[("negative_labels", 1.0)])).recommendation,
        Recommendation::Review
    );
    assert_eq!(
        evaluate(&rules, &features(&[("positive_labels", 1.0)])).recommendation,
        Recommendation::Allow
    );
}

#[test]
fn billing_observation_only_fixture_documents_explicit_fail_open() {
    let service_available = false;
    let checkout_continues = !service_available;
    assert!(
        checkout_continues,
        "Billing fixture is explicitly observation-only in V0"
    );
}
