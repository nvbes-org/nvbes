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
    assert_eq!(
        BudgetThresholds::try_new(0, 2_801, 3_001),
        Err(BudgetPolicyError::ZeroThreshold)
    );
    assert_eq!(
        BudgetThresholds::try_new(2_801, 2_800, 3_001),
        Err(BudgetPolicyError::UnorderedThresholds)
    );
}

#[test]
fn rejects_disable_non_essential_threshold_after_v1_maximum() {
    assert_eq!(
        BudgetThresholds::try_new(2_501, 2_800, 3_000),
        Err(BudgetPolicyError::V1ThresholdTooLate {
            threshold: "disable_non_essential",
            actual_cents: 2_501,
            maximum_cents: 2_500,
        })
    );
}

#[test]
fn rejects_freeze_cost_creation_threshold_after_v1_maximum() {
    assert_eq!(
        BudgetThresholds::try_new(2_500, 2_801, 3_000),
        Err(BudgetPolicyError::V1ThresholdTooLate {
            threshold: "freeze_cost_creation",
            actual_cents: 2_801,
            maximum_cents: 2_800,
        })
    );
}

#[test]
fn rejects_essential_only_threshold_after_v1_maximum() {
    assert_eq!(
        BudgetThresholds::try_new(2_500, 2_800, 3_001),
        Err(BudgetPolicyError::V1ThresholdTooLate {
            threshold: "essential_only",
            actual_cents: 3_001,
            maximum_cents: 3_000,
        })
    );
}

#[test]
fn rejects_delayed_thresholds_before_spend_can_remain_frozen_at_v1_cap() {
    assert_eq!(
        BudgetThresholds::try_new(2_501, 2_801, 3_001),
        Err(BudgetPolicyError::V1ThresholdTooLate {
            threshold: "disable_non_essential",
            actual_cents: 2_501,
            maximum_cents: 2_500,
        })
    );
}

#[test]
fn accepts_thresholds_earlier_than_v1_maxima() {
    let thresholds = BudgetThresholds::try_new(2_000, 2_400, 2_900)
        .expect("earlier thresholds must remain valid");

    assert_eq!(
        thresholds.stage_for(2_000),
        BudgetStage::DisableNonEssential
    );
    assert_eq!(thresholds.stage_for(2_400), BudgetStage::FreezeCostCreation);
    assert_eq!(thresholds.stage_for(2_900), BudgetStage::EssentialOnly);
}

#[test]
fn delayed_threshold_error_explains_the_v1_limit() {
    let error = BudgetThresholds::try_new(2_500, 2_801, 3_000)
        .expect_err("delayed threshold must be rejected");

    assert_eq!(
        error.to_string(),
        "V1 budget threshold freeze_cost_creation cannot exceed 2800 cents (received 2801 cents)"
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
