use crate::config::RetentionConfig;

#[test]
fn retention_classes_remain_independent() {
    let policy = RetentionConfig {
        signals_days: 30,
        evaluations_days: 400,
        labels_days: 500,
        reviews_days: 600,
        audit_days: 730,
    };
    assert!(policy.signals_days < policy.evaluations_days);
    assert!(policy.evaluations_days < policy.audit_days);
}
