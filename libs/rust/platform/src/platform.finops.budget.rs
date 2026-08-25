use serde::{Deserialize, Serialize};
use thiserror::Error;

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
