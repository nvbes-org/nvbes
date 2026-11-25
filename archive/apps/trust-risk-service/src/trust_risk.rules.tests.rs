use super::{RuleOperationError, operator_input_is_valid, validate_operator_input};
#[cfg(feature = "database-tests")]
use super::{activate, rollback, stage};

#[cfg(feature = "database-tests")]
const VALID_RULES: &[u8] = br#"{
    "version":"candidate-v2","feature_version":"features-v1",
    "thresholds":{"challenge":30,"review":60,"deny":85},
    "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":30,"reasons":["velocity.high"],"minimum_recommendation":null}]
}"#;

#[test]
fn operator_mutations_require_actor_and_reason() {
    assert!(validate_operator_input("operator:ada", "activate reviewed rules").is_ok());
    assert!(matches!(
        validate_operator_input("x", "ok"),
        Err(RuleOperationError::InvalidInput)
    ));
    assert!(matches!(
        validate_operator_input("operator:ada", "  "),
        Err(RuleOperationError::InvalidInput)
    ));
    let long_reason = "r".repeat(301);
    assert!(matches!(
        validate_operator_input("operator:ada", &long_reason),
        Err(RuleOperationError::InvalidInput)
    ));
}

#[test]
fn operator_input_accepts_exact_length_boundaries() {
    assert!(operator_input_is_valid("abc", "why"));
    assert!(operator_input_is_valid("abc", &"r".repeat(300)));
    assert!(!operator_input_is_valid("ab", "why"));
    assert!(!operator_input_is_valid("abc", "xy"));
    assert!(!operator_input_is_valid("abc", &"r".repeat(301)));
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn activation_enforces_dual_control_and_audits(pool: sqlx::PgPool) {
    stage(
        &pool,
        "candidate-v2",
        VALID_RULES,
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
    let receipt = activate(
        &pool,
        "candidate-v2",
        "operator:grace",
        "independent approval",
        730,
    )
    .await
    .unwrap();
    assert_eq!(receipt.state, "active");
    assert_eq!(receipt.checksum.len(), 64);
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM trust_risk_audit_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit_count, 2);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn stage_rejects_invalid_payloads_and_conflicts(pool: sqlx::PgPool) {
    assert!(matches!(
        stage(&pool, "bad", b"{", "operator:ada", "broken json", 730).await,
        Err(RuleOperationError::InvalidRules)
    ));
    assert!(matches!(
        stage(
            &pool,
            "other-version",
            VALID_RULES,
            "operator:ada",
            "version mismatch",
            730
        )
        .await,
        Err(RuleOperationError::InvalidRules)
    ));
    stage(
        &pool,
        "candidate-v2",
        VALID_RULES,
        "operator:ada",
        "first stage",
        730,
    )
    .await
    .unwrap();
    assert!(matches!(
        stage(
            &pool,
            "candidate-v2",
            VALID_RULES,
            "operator:ada",
            "duplicate stage",
            730
        )
        .await,
        Err(RuleOperationError::Conflict)
    ));
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn activate_and_rollback_cover_state_machine(pool: sqlx::PgPool) {
    assert!(matches!(
        activate(
            &pool,
            "missing-v1",
            "operator:grace",
            "unknown version",
            730
        )
        .await,
        Err(RuleOperationError::NotFound)
    ));

    let v1 = br#"{
        "version":"rules-v1","feature_version":"features-v1",
        "thresholds":{"challenge":30,"review":60,"deny":85},
        "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":30,"reasons":["velocity.high"],"minimum_recommendation":null}]
    }"#;
    let v2 = br#"{
        "version":"rules-v2","feature_version":"features-v1",
        "thresholds":{"challenge":30,"review":60,"deny":85},
        "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":20},"score_delta":40,"reasons":["velocity.high"],"minimum_recommendation":null}]
    }"#;

    stage(&pool, "rules-v1", v1, "operator:ada", "stage v1", 730)
        .await
        .unwrap();
    activate(&pool, "rules-v1", "operator:grace", "activate v1", 730)
        .await
        .unwrap();
    assert!(matches!(
        activate(&pool, "rules-v1", "operator:ada", "already active", 730).await,
        Err(RuleOperationError::InvalidState)
    ));

    stage(&pool, "rules-v2", v2, "operator:ada", "stage v2", 730)
        .await
        .unwrap();
    activate(&pool, "rules-v2", "operator:grace", "activate v2", 730)
        .await
        .unwrap();

    let rolled = rollback(
        &pool,
        "rules-v1",
        "operator:grace",
        "rollback to prior",
        730,
    )
    .await
    .unwrap();
    assert_eq!(rolled.version, "rules-v1");
    assert_eq!(rolled.state, "active");
}
