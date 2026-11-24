use std::collections::BTreeMap;

use proptest::prelude::*;

use crate::{
    proto::nvbes::trust_risk::v1 as pb,
    rules::{FeatureMap, RuleSet, evaluate},
    signal::{SignalError, SubjectReference},
    types::{Recommendation, RiskBand},
};

const BASELINE_RULESET_JSON: &[u8] = br#"{
  "version": "baseline-v1",
  "feature_version": "features-v1",
  "thresholds": {"challenge": 40, "review": 70, "deny": 90},
  "rules": [
    {
      "code": "automation-high",
      "predicate": {"op": "gte", "feature": "automation_confidence_max_1h", "value": 0.8},
      "score_delta": 75,
      "reasons": ["automation_high"],
      "minimum_recommendation": "challenge"
    },
    {
      "code": "network-high",
      "predicate": {"op": "gte", "feature": "network_risk_score_max", "value": 80.0},
      "score_delta": 50,
      "reasons": ["network_high"],
      "minimum_recommendation": null
    },
    {
      "code": "velocity-anomaly",
      "predicate": {"op": "gt", "feature": "login_attempts_5m", "value": 10.0},
      "score_delta": 30,
      "reasons": ["velocity_high"],
      "minimum_recommendation": "review"
    }
  ]
}"#;

proptest! {
    #[test]
    fn arbitrary_ruleset_json_never_panics(raw in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = RuleSet::from_json(&raw);
    }

    #[test]
    fn evaluate_invariants_and_determinism(
        automation in any::<f64>(),
        network in any::<f64>(),
        velocity in any::<f64>(),
        extra_key in "[a-z_]{1,16}",
        extra_val in any::<f64>(),
    ) {
        let rules = RuleSet::from_json(BASELINE_RULESET_JSON).expect("valid baseline ruleset");
        let mut features: FeatureMap = BTreeMap::new();
        features.insert("automation_confidence_max_1h".to_string(), automation);
        features.insert("network_risk_score_max".to_string(), network);
        features.insert("login_attempts_5m".to_string(), velocity);
        features.insert(extra_key, extra_val);

        let res1 = evaluate(&rules, &features);
        let res2 = evaluate(&rules, &features);

        prop_assert_eq!(&res1, &res2);
        prop_assert!(res1.score <= 100);

        match res1.score {
            90..=100 => {
                prop_assert_eq!(res1.band, RiskBand::Critical);
                prop_assert!(matches!(res1.recommendation, Recommendation::Deny | Recommendation::Review | Recommendation::Challenge));
            }
            70..=89 => prop_assert_eq!(res1.band, RiskBand::High),
            40..=69 => prop_assert_eq!(res1.band, RiskBand::Elevated),
            _ => prop_assert_eq!(res1.band, RiskBand::Low),
        }
    }

    #[test]
    fn subject_reference_try_from_never_panics(
        kind in any::<i32>(),
        namespace in "\\PC{0,100}",
        opaque_id in "\\PC{0,250}",
        scope in any::<i32>(),
        tenant_id in proptest::option::of("\\PC{0,50}"),
    ) {
        let wire = pb::SubjectReference {
            kind,
            namespace,
            opaque_id,
            scope,
            tenant_id,
        };

        let _ = SubjectReference::try_from(wire);
    }

    #[test]
    fn subject_reference_rejects_email_in_opaque_id(
        prefix in "[a-z0-9._-]{3,10}",
        domain in "[a-z0-9._-]{3,10}",
        tld in "[a-z]{2,5}",
    ) {
        let email = format!("{prefix}@{domain}.{tld}");
        let wire = pb::SubjectReference {
            kind: pb::SubjectKind::Principal.into(),
            namespace: "nvbes.identity".to_string(),
            opaque_id: email,
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        };

        let result = SubjectReference::try_from(wire);
        prop_assert_eq!(result, Err(SignalError::ForbiddenSubject));
    }
}
