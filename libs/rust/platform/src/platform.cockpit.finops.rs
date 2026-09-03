use std::collections::HashMap;

use crate::cockpit_model::{FinOpsBudgetSummary, FinOpsCategorySpend};
use crate::finops_budget::{BudgetStage, BudgetThresholds};

pub const DEFAULT_TARGET_CENTS: u32 = 2_000;
pub const DEFAULT_DISABLE_NON_ESSENTIAL_CENTS: u32 = 2_500;
pub const DEFAULT_FREEZE_COST_CREATION_CENTS: u32 = 2_800;
pub const DEFAULT_HARD_LIMIT_CENTS: u32 = 3_000;

pub struct FinOpsMonitor {
    thresholds: BudgetThresholds,
}

impl Default for FinOpsMonitor {
    fn default() -> Self {
        let thresholds = BudgetThresholds::try_new(
            DEFAULT_DISABLE_NON_ESSENTIAL_CENTS,
            DEFAULT_FREEZE_COST_CREATION_CENTS,
            DEFAULT_HARD_LIMIT_CENTS,
        )
        .expect("default budget thresholds must be valid");

        Self { thresholds }
    }
}

impl FinOpsMonitor {
    pub fn new(thresholds: BudgetThresholds) -> Self {
        Self { thresholds }
    }

    pub fn stage_for_spend(&self, spend_cents: u32) -> BudgetStage {
        self.thresholds.stage_for(spend_cents)
    }

    pub fn project_monthly_spend(spend_so_far_cents: u32, day_of_month: u32, days_in_month: u32) -> u32 {
        if day_of_month == 0 || days_in_month == 0 {
            return spend_so_far_cents;
        }
        let daily_average = spend_so_far_cents as f64 / day_of_month as f64;
        (daily_average * days_in_month as f64).round() as u32
    }

    pub fn summarize(
        &self,
        current_spend_cents: u32,
        day_of_month: u32,
        days_in_month: u32,
        category_spends: HashMap<&str, u32>,
    ) -> FinOpsBudgetSummary {
        let stage = self.stage_for_spend(current_spend_cents);
        let projected_monthly_cents =
            Self::project_monthly_spend(current_spend_cents, day_of_month, days_in_month);

        let default_targets = [
            ("domain_dns", 200, 300),
            ("compute", 300, 600),
            ("postgres", 400, 800),
            ("email", 100, 200),
            ("storage_registry", 200, 400),
            ("observability", 0, 200),
            ("safety_margin", 800, 800),
        ];

        let categories = default_targets
            .iter()
            .map(|&(name, target_cents, limit_cents)| {
                let current_cents = *category_spends.get(name).unwrap_or(&0);
                FinOpsCategorySpend {
                    category: name.to_string(),
                    current_cents,
                    target_cents,
                    limit_cents,
                }
            })
            .collect();

        FinOpsBudgetSummary {
            current_spend_cents,
            target_cents: DEFAULT_TARGET_CENTS,
            hard_limit_cents: DEFAULT_HARD_LIMIT_CENTS,
            stage,
            projected_monthly_cents,
            categories,
        }
    }
}

#[cfg(test)]
#[path = "platform.cockpit.finops.tests.rs"]
mod tests;
