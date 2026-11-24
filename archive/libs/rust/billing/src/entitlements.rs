use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::catalog::{AddonGrant, PlanVersion, QuotaDefinition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntitlementStatus {
    Trialing,
    Active,
    Grace,
    PastDue,
    Degraded,
    Suspended,
    Canceled,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementSnapshot {
    pub status: EntitlementStatus,
    pub features: BTreeMap<String, bool>,
    pub quotas: BTreeMap<String, i64>,
    pub billing_locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommercialCreditGrant {
    pub feature_code: Option<String>,
    pub quota_code: Option<String>,
    pub quantity: i64,
    pub expired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionPolicy {
    pub uploads_allowed: bool,
    pub quota_overrides: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementInput {
    pub plan: PlanVersion,
    pub addons: Vec<AddonGrant>,
    pub commercial_credits: Vec<CommercialCreditGrant>,
    pub status: EntitlementStatus,
    pub region_policy: RegionPolicy,
}

pub fn generate_entitlement_snapshot(input: EntitlementInput) -> EntitlementSnapshot {
    let mut features: BTreeMap<String, bool> = input
        .plan
        .features
        .iter()
        .map(|feature| (feature.0.clone(), feature_enabled(input.status)))
        .collect();
    let mut quotas: BTreeMap<String, i64> = input
        .plan
        .quotas
        .iter()
        .map(|quota| entitlement_quota(input.status, quota))
        .collect();

    for addon in &input.addons {
        for feature in &addon.features {
            features.insert(feature.0.clone(), feature_enabled(input.status));
        }
        for quota in &addon.quotas {
            *quotas.entry(quota.code.clone()).or_default() += quota.included_quantity;
        }
    }

    for credit in input
        .commercial_credits
        .iter()
        .filter(|credit| !credit.expired)
    {
        if let Some(feature_code) = &credit.feature_code {
            features.insert(feature_code.clone(), feature_enabled(input.status));
        }
        if let Some(quota_code) = &credit.quota_code {
            *quotas.entry(quota_code.clone()).or_default() += credit.quantity;
        }
    }

    for (code, quantity) in &input.region_policy.quota_overrides {
        quotas.insert(code.clone(), *quantity);
    }

    if !input.region_policy.uploads_allowed {
        features.insert("upload".to_string(), false);
    }

    EntitlementSnapshot {
        status: input.status,
        features,
        quotas,
        billing_locked: matches!(
            input.status,
            EntitlementStatus::PastDue
                | EntitlementStatus::Degraded
                | EntitlementStatus::Suspended
                | EntitlementStatus::Canceled
                | EntitlementStatus::Ended
        ),
    }
}

pub fn entitlement_change_event_type() -> &'static str {
    "billing.entitlement.changed"
}

fn entitlement_quota(status: EntitlementStatus, quota: &QuotaDefinition) -> (String, i64) {
    let quantity = if status == EntitlementStatus::Trialing && quota.code != "trial_storage_gb" {
        0
    } else {
        quota.included_quantity
    };
    (quota.code.clone(), quantity)
}

fn feature_enabled(status: EntitlementStatus) -> bool {
    !matches!(
        status,
        EntitlementStatus::Suspended | EntitlementStatus::Canceled | EntitlementStatus::Ended
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AddonGrant, FeatureCode, QuotaDefinition};
    use uuid::Uuid;

    #[test]
    fn suspended_subscription_locks_features() {
        let plan = PlanVersion {
            id: Uuid::nil(),
            plan_code: "team".to_string(),
            version: 1,
            features: vec![FeatureCode("share_links".to_string())],
            quotas: vec![QuotaDefinition {
                code: "storage_gb".to_string(),
                unit: "gb".to_string(),
                included_quantity: 100,
            }],
            price: None,
        };

        let snapshot = generate_entitlement_snapshot(EntitlementInput {
            plan,
            addons: Vec::new(),
            commercial_credits: Vec::new(),
            status: EntitlementStatus::Suspended,
            region_policy: RegionPolicy {
                uploads_allowed: true,
                quota_overrides: BTreeMap::new(),
            },
        });
        assert!(snapshot.billing_locked);
        assert_eq!(snapshot.features.get("share_links"), Some(&false));
    }

    #[test]
    fn addon_credit_and_region_policy_shape_entitlements() {
        let plan = PlanVersion {
            id: Uuid::nil(),
            plan_code: "team".to_string(),
            version: 1,
            features: vec![FeatureCode("upload".to_string())],
            quotas: vec![QuotaDefinition {
                code: "storage_gb".to_string(),
                unit: "gb".to_string(),
                included_quantity: 100,
            }],
            price: None,
        };
        let mut quota_overrides = BTreeMap::new();
        quota_overrides.insert("egress_gb".to_string(), 250);

        let snapshot = generate_entitlement_snapshot(EntitlementInput {
            plan,
            addons: vec![AddonGrant {
                code: "extra_storage".to_string(),
                features: vec![FeatureCode("audit_export".to_string())],
                quotas: vec![QuotaDefinition {
                    code: "storage_gb".to_string(),
                    unit: "gb".to_string(),
                    included_quantity: 50,
                }],
            }],
            commercial_credits: vec![CommercialCreditGrant {
                feature_code: None,
                quota_code: Some("storage_gb".to_string()),
                quantity: 25,
                expired: false,
            }],
            status: EntitlementStatus::Active,
            region_policy: RegionPolicy {
                uploads_allowed: false,
                quota_overrides,
            },
        });

        assert_eq!(snapshot.features.get("upload"), Some(&false));
        assert_eq!(snapshot.features.get("audit_export"), Some(&true));
        assert_eq!(snapshot.quotas.get("storage_gb"), Some(&175));
        assert_eq!(snapshot.quotas.get("egress_gb"), Some(&250));
        assert_eq!(
            entitlement_change_event_type(),
            "billing.entitlement.changed"
        );
    }

    #[test]
    fn trialing_status_zeros_non_trial_quotas_and_grants_feature_credits() {
        let plan = PlanVersion {
            id: Uuid::nil(),
            plan_code: "trial".to_string(),
            version: 1,
            features: vec![FeatureCode("share_links".to_string())],
            quotas: vec![
                QuotaDefinition {
                    code: "storage_gb".to_string(),
                    unit: "gb".to_string(),
                    included_quantity: 100,
                },
                QuotaDefinition {
                    code: "trial_storage_gb".to_string(),
                    unit: "gb".to_string(),
                    included_quantity: 5,
                },
            ],
            price: None,
        };

        let snapshot = generate_entitlement_snapshot(EntitlementInput {
            plan,
            addons: Vec::new(),
            commercial_credits: vec![
                CommercialCreditGrant {
                    feature_code: Some("priority_support".to_string()),
                    quota_code: None,
                    quantity: 1,
                    expired: false,
                },
                CommercialCreditGrant {
                    feature_code: None,
                    quota_code: Some("api_calls".to_string()),
                    quantity: 10,
                    expired: true,
                },
            ],
            status: EntitlementStatus::Trialing,
            region_policy: RegionPolicy {
                uploads_allowed: true,
                quota_overrides: BTreeMap::new(),
            },
        });

        assert_eq!(snapshot.quotas.get("storage_gb"), Some(&0));
        assert_eq!(snapshot.quotas.get("trial_storage_gb"), Some(&5));
        assert_eq!(snapshot.features.get("priority_support"), Some(&true));
        assert!(!snapshot.quotas.contains_key("api_calls"));
        assert!(!snapshot.billing_locked);
    }
}
