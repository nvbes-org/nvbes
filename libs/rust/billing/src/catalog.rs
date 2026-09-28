use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::pricing::PriceVersion;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureCode(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanVersion {
    pub id: Uuid,
    pub plan_code: String,
    pub version: i32,
    pub features: Vec<FeatureCode>,
    pub quotas: Vec<QuotaDefinition>,
    pub price: Option<PriceVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonGrant {
    pub code: String,
    pub features: Vec<FeatureCode>,
    pub quotas: Vec<QuotaDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaDefinition {
    pub code: String,
    pub unit: String,
    pub included_quantity: i64,
}

impl PlanVersion {
    pub fn includes_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|code| code.0 == feature)
    }

    pub fn quota(&self, quota_code: &str) -> Option<i64> {
        self.quotas
            .iter()
            .find(|quota| quota.code == quota_code)
            .map(|quota| quota.included_quantity)
    }
}

#[cfg(test)]
#[path = "catalog.tests.rs"]
mod tests;
