use super::{FeatureCode, PlanVersion, QuotaDefinition};
use uuid::Uuid;

fn sample_plan() -> PlanVersion {
    PlanVersion {
        id: Uuid::new_v4(),
        plan_code: "standard_monthly".into(),
        version: 1,
        features: vec![FeatureCode("billing.portal".into())],
        quotas: vec![QuotaDefinition {
            code: "seats".into(),
            unit: "count".into(),
            included_quantity: 5,
        }],
        price: None,
    }
}

#[test]
fn includes_feature_and_quota_lookups() {
    let plan = sample_plan();
    assert!(plan.includes_feature("billing.portal"));
    assert!(!plan.includes_feature("cloud.drive"));
    assert_eq!(plan.quota("seats"), Some(5));
    assert_eq!(plan.quota("missing"), None);
}
