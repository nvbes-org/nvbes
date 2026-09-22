use super::*;

#[test]
fn provides_all_seven_mandatory_procedures() {
    let procedures = DegradedModeRegistry::all_procedures();
    assert_eq!(procedures.len(), 7);
    for p in &procedures {
        assert!(!p.operator_checklist.is_empty());
        assert!(!p.verification_steps.is_empty());
        assert!(!p.title.is_empty());
        assert!(!p.severity.is_empty());
        assert!(!p.automatic_fallback.is_empty());
        assert_eq!(
            p.scenario,
            DegradedModeRegistry::get_procedure(p.scenario).scenario
        );
    }
}

#[test]
fn severity_matches_scenario_criticality() {
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::IdentityUnavailable).severity,
        "SEV1"
    );
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::PostgresInRestore).severity,
        "SEV1"
    );
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::TrustRiskUnavailable).severity,
        "SEV3"
    );
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::FinOpsThresholdExceeded).severity,
        "SEV2"
    );
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::AccountUnavailable).severity,
        "SEV2"
    );
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::StripeUnavailable).severity,
        "SEV2"
    );
    assert_eq!(
        DegradedModeRegistry::get_procedure(DegradedScenario::EmailDelayed).severity,
        "SEV2"
    );
}

#[test]
fn degraded_scenario_round_trips_through_serde() {
    for scenario in [
        DegradedScenario::IdentityUnavailable,
        DegradedScenario::AccountUnavailable,
        DegradedScenario::StripeUnavailable,
        DegradedScenario::EmailDelayed,
        DegradedScenario::TrustRiskUnavailable,
        DegradedScenario::PostgresInRestore,
        DegradedScenario::FinOpsThresholdExceeded,
    ] {
        let encoded = serde_json::to_string(&scenario).expect("serialize");
        let decoded: DegradedScenario = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, scenario);
    }
    assert_eq!(
        serde_json::to_string(&DegradedScenario::EmailDelayed).unwrap(),
        "\"email_delayed\""
    );
}
