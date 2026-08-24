use super::{RuleOperationError, validate_operator_input};
#[cfg(feature = "database-tests")]
use super::{activate, stage};

#[test]
fn operator_mutations_require_actor_and_reason() {
    assert!(validate_operator_input("operator:ada", "activate reviewed rules").is_ok());
    assert!(matches!(
        validate_operator_input("x", "ok"),
        Err(RuleOperationError::InvalidInput)
    ));
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn activation_enforces_dual_control_and_audits(pool: sqlx::PgPool) {
    let json = br#"{
        "version":"candidate-v2","feature_version":"features-v1",
        "thresholds":{"challenge":30,"review":60,"deny":85},
        "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":30,"reasons":["velocity.high"],"minimum_recommendation":null}]
    }"#;
    stage(
        &pool,
        "candidate-v2",
        json,
        "operator:ada",
        "reviewed candidate",
        730,
    )
    .await
    .unwrap();
    assert!(matches!(
        activate(
            &pool,
            "candidate-v2",
            "operator:ada",
            "self activation",
            730
        )
        .await
        .unwrap_err(),
        RuleOperationError::DualControl
    ));
    activate(
        &pool,
        "candidate-v2",
        "operator:grace",
        "independent approval",
        730,
    )
    .await
    .unwrap();
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM trust_risk_audit_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit_count, 2);
}
