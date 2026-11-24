use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use serde_json::json;

use crate::rules::{RuleSet, evaluate};
use crate::types::{Recommendation, RiskBand};

fn rules(deltas: &[i16], thresholds: [u8; 3]) -> RuleSet {
    let rules: Vec<_> = deltas
        .iter()
        .enumerate()
        .map(|(index, delta)| {
            json!({
                "code": format!("rule-{index}"),
                "predicate": {"op": "gte", "feature": "risk", "value": index},
                "score_delta": delta,
                "reasons": [format!("reason-{index}")],
                "minimum_recommendation": null
            })
        })
        .collect();
    RuleSet::from_json(&serde_json::to_vec(&json!({
        "version": "properties-v1", "feature_version": "features-v1",
        "thresholds": {"challenge": thresholds[0], "review": thresholds[1], "deny": thresholds[2]},
        "rules": rules
    })).unwrap()).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 2048, rng_seed: RngSeed::Fixed(20260912), ..ProptestConfig::default()
    })]

    #[test]
    fn properties_score_is_clamped_sum_of_matching_rules(
        deltas in prop::collection::vec(-100_i16..=100, 1..=256),
        matching in 0_usize..=256,
    ) {
        let rules = rules(&deltas, [40, 70, 90]);
        let features = BTreeMap::from([("risk".to_string(), matching as f64 - 0.5)]);
        let result = evaluate(&rules, &features);
        let sum: i32 = deltas.iter().take(matching).map(|&delta| i32::from(delta)).sum();
        prop_assert_eq!(result.score, sum.clamp(0, 100) as u8);
        prop_assert_eq!(result.reason_codes.len(), matching.min(deltas.len()));
        prop_assert_eq!(&result, &evaluate(&rules, &features));
        let reasons: Vec<_> = (0..matching.min(deltas.len())).map(|i| format!("reason-{i}")).collect();
        prop_assert_eq!(result.reason_codes, reasons);
    }

    #[test]
    fn properties_positive_evidence_cannot_reduce_risk(
        deltas in prop::collection::vec(0_i16..=100, 1..=32),
        lower in 0_u8..32,
        increase in 0_u8..32,
    ) {
        let rules = rules(&deltas, [40, 70, 90]);
        let low = evaluate(&rules, &BTreeMap::from([("risk".into(), f64::from(lower))]));
        let high = evaluate(&rules, &BTreeMap::from([("risk".into(), f64::from(lower) + f64::from(increase))]));
        prop_assert!(high.score >= low.score);
        prop_assert!(high.recommendation >= low.recommendation);
    }

    #[test]
    fn properties_thresholds_choose_the_strictest_matching_band(
        mut thresholds in prop::array::uniform3(0_u8..=100),
        score in 0_i16..=100,
    ) {
        thresholds.sort_unstable();
        let rules = rules(&[score], thresholds);
        let result = evaluate(&rules, &BTreeMap::from([("risk".into(), 0.0)]));
        let index = thresholds.iter().filter(|&&threshold| score >= i16::from(threshold)).count();
        prop_assert_eq!(result.recommendation, [Recommendation::Allow, Recommendation::Challenge,
            Recommendation::Review, Recommendation::Deny][index]);
        prop_assert_eq!(result.band, [RiskBand::Low, RiskBand::Elevated, RiskBand::High, RiskBand::Critical][index]);
    }

    #[test]
    fn properties_rule_serialization_preserves_evaluation(
        deltas in prop::collection::vec(-100_i16..=100, 1..=32),
        risk in -10_i16..100,
    ) {
        let original = rules(&deltas, [40, 70, 90]);
        let decoded = RuleSet::from_json(&serde_json::to_vec(&original).unwrap()).unwrap();
        let features = BTreeMap::from([("risk".into(), f64::from(risk))]);
        prop_assert_eq!(evaluate(&original, &features), evaluate(&decoded, &features));
    }
}
