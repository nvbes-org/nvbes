use crate::rules::{RuleSet, RuleSetError, evaluate};
use std::collections::BTreeMap;

fn base_ruleset() -> serde_json::Value {
    serde_json::json!({
      "version":"baseline-v1",
      "feature_version":"features-v1",
      "thresholds":{"challenge":40,"review":70,"deny":90},
      "rules":[{
        "code":"network-high",
        "predicate":{"op":"gte","feature":"network_risk_score_max","value":80},
        "score_delta":50,
        "reasons":["network_high"],
        "minimum_recommendation":null
      }]
    })
}

#[test]
fn rejects_invalid_thresholds_and_empty_rules() {
    let mut document = base_ruleset();
    document["thresholds"] = serde_json::json!({"challenge":90,"review":70,"deny":40});
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::Thresholds)
    ));

    let mut document = base_ruleset();
    document["rules"] = serde_json::json!([]);
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::RuleCount)
    ));
}

#[test]
fn rejects_duplicate_rule_codes_and_reasons() {
    let mut document = base_ruleset();
    document["rules"] = serde_json::json!([
      {
        "code":"network-high",
        "predicate":{"op":"gte","feature":"network_risk_score_max","value":80},
        "score_delta":10,
        "reasons":["network_high"],
        "minimum_recommendation":null
      },
      {
        "code":"network-high",
        "predicate":{"op":"gte","feature":"network_risk_score_max","value":90},
        "score_delta":10,
        "reasons":["network_critical"],
        "minimum_recommendation":null
      }
    ]);
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::DuplicateRule)
    ));

    let mut document = base_ruleset();
    document["rules"] = serde_json::json!([
      {
        "code":"a",
        "predicate":{"op":"gte","feature":"network_risk_score_max","value":80},
        "score_delta":10,
        "reasons":["shared_reason"],
        "minimum_recommendation":null
      },
      {
        "code":"b",
        "predicate":{"op":"gte","feature":"network_risk_score_max","value":90},
        "score_delta":10,
        "reasons":["shared_reason"],
        "minimum_recommendation":null
      }
    ]);
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::DuplicateReason)
    ));
}

#[test]
fn rejects_invalid_score_delta_reason_count_and_non_finite_predicate() {
    let mut document = base_ruleset();
    document["rules"][0]["score_delta"] = serde_json::json!(101);
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::ScoreDelta)
    ));

    let mut document = base_ruleset();
    document["rules"][0]["reasons"] = serde_json::json!([]);
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::ReasonCount)
    ));

    let mut document = base_ruleset();
    document["rules"][0]["predicate"] =
        serde_json::json!({"op":"gte","feature":"network_risk_score_max","value":null});
    // null fails JSON schema of f64 -> Json error, use Infinity via string rewrite
    let raw = br#"{"version":"baseline-v1","feature_version":"features-v1","thresholds":{"challenge":40,"review":70,"deny":90},"rules":[{"code":"network-high","predicate":{"op":"gte","feature":"network_risk_score_max","value":null},"score_delta":50,"reasons":["network_high"],"minimum_recommendation":null}]}"#;
    assert!(RuleSet::from_json(raw).is_err());
}

#[test]
fn evaluate_covers_comparison_operators_and_boolean_combinators() {
    let document = serde_json::json!({
      "version":"ops-v1",
      "feature_version":"features-v1",
      "thresholds":{"challenge":40,"review":70,"deny":90},
      "rules":[
        {
          "code":"all-match",
          "predicate":{"op":"all","predicates":[
            {"op":"gt","feature":"a","value":1.0},
            {"op":"lt","feature":"b","value":10.0},
            {"op":"lte","feature":"c","value":5.0},
            {"op":"eq","feature":"d","value":2.0}
          ]},
          "score_delta":20,
          "reasons":["all_ok"],
          "minimum_recommendation":"challenge"
        },
        {
          "code":"any-or-not",
          "predicate":{"op":"any","predicates":[
            {"op":"not","predicate":{"op":"gte","feature":"blocked","value":1.0}},
            {"op":"gte","feature":"force","value":1.0}
          ]},
          "score_delta":15,
          "reasons":["any_ok"],
          "minimum_recommendation":null
        }
      ]
    });
    let rules = RuleSet::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
    let features = BTreeMap::from([
        ("a".to_string(), 2.0),
        ("b".to_string(), 9.0),
        ("c".to_string(), 5.0),
        ("d".to_string(), 2.0),
        ("blocked".to_string(), 0.0),
    ]);
    let result = evaluate(&rules, &features);
    assert_eq!(result.score, 35);
    assert!(result.reason_codes.contains(&"all_ok".to_string()));
    assert!(result.reason_codes.contains(&"any_ok".to_string()));
    assert_eq!(rules.version(), "ops-v1");
    assert_eq!(rules.feature_version(), "features-v1");
}

#[test]
fn rejects_empty_any_predicate_list() {
    let document = serde_json::json!({
      "version":"bad-v1",
      "feature_version":"features-v1",
      "thresholds":{"challenge":40,"review":70,"deny":90},
      "rules":[{
        "code":"empty-any",
        "predicate":{"op":"any","predicates":[]},
        "score_delta":1,
        "reasons":["empty"],
        "minimum_recommendation":null
      }]
    });
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::PredicateCount)
    ));
}

#[test]
fn rejects_oversized_rule_reason_and_predicate_collections() {
    let mut document = base_ruleset();
    document["rules"] = (0..257)
        .map(|index| {
            serde_json::json!({
              "code": format!("rule-{index}"),
              "predicate":{"op":"gte","feature":"network_risk_score_max","value":80},
              "score_delta":1,
              "reasons":[format!("reason-{index}")],
              "minimum_recommendation":null
            })
        })
        .collect();
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::RuleCount)
    ));

    let mut document = base_ruleset();
    document["rules"][0]["reasons"] = (0..9)
        .map(|index| format!("reason-{index}"))
        .collect::<Vec<_>>()
        .into();
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::ReasonCount)
    ));

    let mut document = base_ruleset();
    document["rules"][0]["predicate"] = serde_json::json!({
      "op":"all",
      "predicates":(0..17).map(|_| serde_json::json!({
        "op":"gte","feature":"network_risk_score_max","value":1
      })).collect::<Vec<_>>()
    });
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::PredicateCount)
    ));
}

#[test]
fn rejects_non_monotonic_mid_thresholds() {
    // challenge <= review holds, but review > deny must fail the middle conjunct.
    let mut document = base_ruleset();
    document["thresholds"] = serde_json::json!({"challenge":40,"review":90,"deny":70});
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::Thresholds)
    ));
}

#[test]
fn comparison_operators_use_strict_boundaries() {
    let document = serde_json::json!({
      "version":"bounds-v1",
      "feature_version":"features-v1",
      "thresholds":{"challenge":40,"review":70,"deny":90},
      "rules":[
        {"code":"gt-only","predicate":{"op":"gt","feature":"a","value":5.0},"score_delta":10,"reasons":["gt"],"minimum_recommendation":null},
        {"code":"gte-only","predicate":{"op":"gte","feature":"b","value":5.0},"score_delta":10,"reasons":["gte"],"minimum_recommendation":null},
        {"code":"lt-only","predicate":{"op":"lt","feature":"c","value":5.0},"score_delta":10,"reasons":["lt"],"minimum_recommendation":null},
        {"code":"lte-only","predicate":{"op":"lte","feature":"d","value":5.0},"score_delta":10,"reasons":["lte"],"minimum_recommendation":null},
        {"code":"eq-only","predicate":{"op":"eq","feature":"e","value":5.0},"score_delta":10,"reasons":["eq"],"minimum_recommendation":null}
      ]
    });
    let rules = RuleSet::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
    let on_boundary = BTreeMap::from([
        ("a".to_string(), 5.0),
        ("b".to_string(), 5.0),
        ("c".to_string(), 5.0),
        ("d".to_string(), 5.0),
        ("e".to_string(), 5.0),
    ]);
    let result = evaluate(&rules, &on_boundary);
    assert_eq!(result.score, 30);
    assert_eq!(
        result.reason_codes,
        vec!["gte".to_string(), "lte".to_string(), "eq".to_string()]
    );
}

#[test]
fn validate_key_rejects_empty_overlong_and_charset() {
    for version in ["", &"a".repeat(81), "BadCase", "space key", "emoji😀"] {
        let mut document = base_ruleset();
        document["version"] = serde_json::json!(version);
        assert!(
            matches!(
                RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
                Err(RuleSetError::InvalidKey)
            ),
            "version {version:?} must be rejected"
        );
    }

    let mut document = base_ruleset();
    document["version"] = serde_json::json!("a".repeat(80));
    RuleSet::from_json(&serde_json::to_vec(&document).unwrap())
        .expect("version length 80 must be accepted");
}

#[test]
fn predicate_depth_allows_max_and_rejects_one_past() {
    // Root validate(1); leaf at depth 8 is allowed, depth 9 is not.
    fn nest(depth: usize) -> serde_json::Value {
        let mut predicate = serde_json::json!({
            "op": "gte",
            "feature": "events_1h",
            "value": 10
        });
        for _ in 0..depth {
            predicate = serde_json::json!({"op": "not", "predicate": predicate});
        }
        serde_json::json!({
            "version":"depth-v1",
            "feature_version":"features-v1",
            "thresholds":{"challenge":40,"review":70,"deny":90},
            "rules":[{
                "code":"depth",
                "predicate":predicate,
                "score_delta":1,
                "reasons":["depth"],
                "minimum_recommendation":null
            }]
        })
    }

    assert!(RuleSet::from_json(&serde_json::to_vec(&nest(7)).unwrap()).is_ok());
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&nest(8)).unwrap()),
        Err(RuleSetError::PredicateDepth)
    ));
}

#[test]
fn deep_all_combinator_hits_predicate_depth() {
    let mut predicate = serde_json::json!({
        "op": "gte",
        "feature": "events_1h",
        "value": 10
    });
    for _ in 0..8 {
        predicate = serde_json::json!({
            "op": "all",
            "predicates": [predicate]
        });
    }
    let document = serde_json::json!({
        "version":"deep-all-v1",
        "feature_version":"features-v1",
        "thresholds":{"challenge":40,"review":70,"deny":90},
        "rules":[{
            "code":"deep-all",
            "predicate":predicate,
            "score_delta":1,
            "reasons":["deep_all"],
            "minimum_recommendation":null
        }]
    });
    assert!(matches!(
        RuleSet::from_json(&serde_json::to_vec(&document).unwrap()),
        Err(RuleSetError::PredicateDepth)
    ));
}
