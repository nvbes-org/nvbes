use serde::{Deserialize, Serialize};
use thiserror::Error;

const V1_DISABLE_NON_ESSENTIAL_MAX_CENTS: u32 = 2_500;
const V1_FREEZE_COST_CREATION_MAX_CENTS: u32 = 2_800;
const V1_ESSENTIAL_ONLY_MAX_CENTS: u32 = 3_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStage {
    Normal,
    DisableNonEssential,
    FreezeCostCreation,
    EssentialOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetThresholds {
    disable_non_essential_cents: u32,
    freeze_cost_creation_cents: u32,
    essential_only_cents: u32,
}

#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum BudgetPolicyError {
    #[error("budget thresholds must be greater than zero")]
    ZeroThreshold,
    #[error("budget thresholds must be strictly increasing")]
    UnorderedThresholds,
    #[error(
        "V1 budget threshold {threshold} cannot exceed {maximum_cents} cents (received {actual_cents} cents)"
    )]
    V1ThresholdTooLate {
        threshold: &'static str,
        actual_cents: u32,
        maximum_cents: u32,
    },
}

impl BudgetThresholds {
    pub fn try_new(
        disable_non_essential_cents: u32,
        freeze_cost_creation_cents: u32,
        essential_only_cents: u32,
    ) -> Result<Self, BudgetPolicyError> {
        if disable_non_essential_cents == 0
            || freeze_cost_creation_cents == 0
            || essential_only_cents == 0
        {
            return Err(BudgetPolicyError::ZeroThreshold);
        }
        if disable_non_essential_cents >= freeze_cost_creation_cents
            || freeze_cost_creation_cents >= essential_only_cents
        {
            return Err(BudgetPolicyError::UnorderedThresholds);
        }
        if disable_non_essential_cents > V1_DISABLE_NON_ESSENTIAL_MAX_CENTS {
            return Err(BudgetPolicyError::V1ThresholdTooLate {
                threshold: "disable_non_essential",
                actual_cents: disable_non_essential_cents,
                maximum_cents: V1_DISABLE_NON_ESSENTIAL_MAX_CENTS,
            });
        }
        if freeze_cost_creation_cents > V1_FREEZE_COST_CREATION_MAX_CENTS {
            return Err(BudgetPolicyError::V1ThresholdTooLate {
                threshold: "freeze_cost_creation",
                actual_cents: freeze_cost_creation_cents,
                maximum_cents: V1_FREEZE_COST_CREATION_MAX_CENTS,
            });
        }
        if essential_only_cents > V1_ESSENTIAL_ONLY_MAX_CENTS {
            return Err(BudgetPolicyError::V1ThresholdTooLate {
                threshold: "essential_only",
                actual_cents: essential_only_cents,
                maximum_cents: V1_ESSENTIAL_ONLY_MAX_CENTS,
            });
        }

        Ok(Self {
            disable_non_essential_cents,
            freeze_cost_creation_cents,
            essential_only_cents,
        })
    }

    pub fn stage_for(self, spend_cents: u32) -> BudgetStage {
        if spend_cents >= self.essential_only_cents {
            BudgetStage::EssentialOnly
        } else if spend_cents >= self.freeze_cost_creation_cents {
            BudgetStage::FreezeCostCreation
        } else if spend_cents >= self.disable_non_essential_cents {
            BudgetStage::DisableNonEssential
        } else {
            BudgetStage::Normal
        }
    }
}

#[cfg(test)]
#[path = "platform.finops.budget.tests.rs"]
mod tests;
