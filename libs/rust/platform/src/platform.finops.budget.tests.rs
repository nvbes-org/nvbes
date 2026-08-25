use super::{BudgetPolicyError, BudgetStage, BudgetThresholds};

#[test]
fn selects_every_v1_budget_stage_at_its_boundary() {
    let thresholds =
        BudgetThresholds::try_new(2_500, 2_800, 3_000).expect("approved thresholds must be valid");

    assert_eq!(thresholds.stage_for(0), BudgetStage::Normal);
    assert_eq!(thresholds.stage_for(2_499), BudgetStage::Normal);
    assert_eq!(
        thresholds.stage_for(2_500),
        BudgetStage::DisableNonEssential
    );
    assert_eq!(thresholds.stage_for(2_800), BudgetStage::FreezeCostCreation);
    assert_eq!(thresholds.stage_for(3_000), BudgetStage::EssentialOnly);
    assert_eq!(thresholds.stage_for(8_000), BudgetStage::EssentialOnly);
}

#[test]
fn rejects_zero_or_unordered_thresholds() {
    assert_eq!(
        BudgetThresholds::try_new(0, 2_800, 3_000),
        Err(BudgetPolicyError::ZeroThreshold)
    );
    assert_eq!(
        BudgetThresholds::try_new(2_800, 2_800, 3_000),
        Err(BudgetPolicyError::UnorderedThresholds)
    );
    assert_eq!(
        BudgetThresholds::try_new(2_500, 3_100, 3_000),
        Err(BudgetPolicyError::UnorderedThresholds)
    );
}

#[test]
fn stage_names_match_the_repository_contract() {
    assert_eq!(
        serde_json::to_string(&BudgetStage::DisableNonEssential).expect("stage must serialize"),
        "\"disable_non_essential\""
    );
    assert_eq!(
        serde_json::from_str::<BudgetStage>("\"freeze_cost_creation\"")
            .expect("stage must deserialize"),
        BudgetStage::FreezeCostCreation
    );
}
