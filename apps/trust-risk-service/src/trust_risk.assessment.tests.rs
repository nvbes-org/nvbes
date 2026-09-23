use std::collections::HashMap;

use nvbes_trust_risk::{assessment::Assessment, proto::nvbes::trust_risk::v1 as pb};

#[cfg(feature = "database-tests")]
use super::assess;
use super::fingerprint;
#[cfg(feature = "database-tests")]
use crate::assessment_error::AssessmentPersistenceError;

fn request() -> pb::AssessRiskRequest {
    let subject = pb::SubjectReference {
        kind: pb::SubjectKind::Principal.into(),
        namespace: "nvbes.identity".to_string(),
        opaque_id: "principal:018f7f2d-fc7d".to_string(),
        scope: pb::DataScope::Regional.into(),
        tenant_id: None,
    };
    let signal = pb::RiskSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d301".to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:principal:example".to_string(),
        subjects: vec![subject.clone()],
        attributes: HashMap::from([(
            "risk_score".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::UnsignedValue(90)),
            },
        )]),
        context: None,
        scope: pb::DataScope::Regional.into(),
    };
    pb::AssessRiskRequest {
        context: None,
        producer: "billing-checkout-fixture".to_string(),
        assessment_key: "checkout:018f7f2d-fc7d-7b7a".to_string(),
        operation_class: "payment.checkout".to_string(),
        subjects: vec![subject],
        instantaneous_signals: vec![signal],
    }
}

#[test]
fn assessment_fingerprint_is_stable_and_content_sensitive() {
    let first = Assessment::try_from(request()).unwrap();
    let second = Assessment::try_from(request()).unwrap();
    assert_eq!(fingerprint(&first), fingerprint(&second));
    let mut changed = request();
    changed.operation_class = "identity.login".to_string();
    assert_ne!(
        fingerprint(&first),
        fingerprint(&Assessment::try_from(changed).unwrap())
    );
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn assessment_persists_evidence_and_replays_original_result(pool: sqlx::PgPool) {
    let wire = request();
    let domain = Assessment::try_from(wire.clone()).unwrap();
    let first = assess(&pool, &wire, &domain, 400, 30, 400).await.unwrap();
    assert!(!first.duplicate);
    assert_eq!(first.rule_set_version, "baseline-v1");

    let duplicate = assess(&pool, &wire, &domain, 400, 30, 400).await.unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.id, first.id);

    let mut changed_wire = wire;
    changed_wire.operation_class = "identity.login".to_string();
    let changed = Assessment::try_from(changed_wire.clone()).unwrap();
    assert!(matches!(
        assess(&pool, &changed_wire, &changed, 400, 30, 400)
            .await
            .unwrap_err(),
        AssessmentPersistenceError::Conflict
    ));
    let signal_count: i64 = sqlx::query_scalar("SELECT count(*) FROM trust_risk_signals")
        .fetch_one(&pool)
        .await
        .unwrap();
    let evaluation_count: i64 = sqlx::query_scalar("SELECT count(*) FROM trust_risk_evaluations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((signal_count, evaluation_count), (1, 1));
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn assessment_rejects_feature_version_mismatch_and_opens_review(pool: sqlx::PgPool) {
    sqlx::query(
        "UPDATE trust_risk_rule_sets SET feature_version = 'features-mismatch' WHERE state = 'active'",
    )
    .execute(&pool)
    .await
    .expect("mismatch feature version column");
    let wire = request();
    let domain = Assessment::try_from(wire.clone()).unwrap();
    assert!(matches!(
        assess(&pool, &wire, &domain, 400, 30, 400)
            .await
            .unwrap_err(),
        AssessmentPersistenceError::InvalidRules
    ));

    sqlx::query(
        "UPDATE trust_risk_rule_sets SET feature_version = 'features-v1' WHERE state = 'active'",
    )
    .execute(&pool)
    .await
    .expect("restore feature version");

    // Seed projected features so baseline rules push the score into the review band.
    sqlx::query(
        r#"
        INSERT INTO trust_risk_feature_state (
            subject_kind, namespace, opaque_id, feature_version, features,
            event_watermark, expires_at
        ) VALUES (
            1, 'nvbes.identity', 'principal:018f7f2d-fc7d', 'features-v1',
            '{"events_1h":10,"negative_labels":1}'::jsonb,
            clock_timestamp(), clock_timestamp() + INTERVAL '25 hours'
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed features");

    let mut review_wire = request();
    review_wire.assessment_key = "checkout:review-band".to_string();
    review_wire.instantaneous_signals.clear();
    let review_domain = Assessment::try_from(review_wire.clone()).unwrap();
    let stored = assess(&pool, &review_wire, &review_domain, 400, 30, 400)
        .await
        .expect("review assessment");
    assert_eq!(
        stored.recommendation,
        nvbes_trust_risk::types::Recommendation::Review
    );
    let review_cases: i64 =
        sqlx::query_scalar("SELECT count(*) FROM trust_risk_review_cases WHERE evaluation_id = $1")
            .bind(stored.id)
            .fetch_one(&pool)
            .await
            .expect("review case");
    assert_eq!(review_cases, 1);
}
