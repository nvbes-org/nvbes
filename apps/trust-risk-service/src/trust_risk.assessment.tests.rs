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
    assert_eq!(
        first.expires_at,
        first.evaluated_at + chrono::Duration::days(400),
        "evaluation retention must add days (`+` not `-`)"
    );
    let feature_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM trust_risk_evaluation_features WHERE evaluation_id = $1",
    )
    .bind(first.id)
    .fetch_one(&pool)
    .await
    .expect("feature count");
    assert!(
        feature_count > 0,
        "persist_snapshot must write evaluation features"
    );
    let reason_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM trust_risk_evaluation_reasons WHERE evaluation_id = $1",
    )
    .bind(first.id)
    .fetch_one(&pool)
    .await
    .expect("reason count");
    assert!(
        reason_count > 0,
        "persist_snapshot must write evaluation reasons"
    );

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

#[test]
fn parse_band_and_recommendation_cover_all_ledger_values() {
    use nvbes_trust_risk::types::{Recommendation, RiskBand};

    assert_eq!(super::parse_band("low").unwrap(), RiskBand::Low);
    assert_eq!(super::parse_band("elevated").unwrap(), RiskBand::Elevated);
    assert_eq!(super::parse_band("high").unwrap(), RiskBand::High);
    assert_eq!(super::parse_band("critical").unwrap(), RiskBand::Critical);
    assert!(super::parse_band("nope").is_err());

    assert_eq!(
        super::parse_recommendation("allow").unwrap(),
        Recommendation::Allow
    );
    assert_eq!(
        super::parse_recommendation("challenge").unwrap(),
        Recommendation::Challenge
    );
    assert_eq!(
        super::parse_recommendation("review").unwrap(),
        Recommendation::Review
    );
    assert_eq!(
        super::parse_recommendation("deny").unwrap(),
        Recommendation::Deny
    );
    assert!(super::parse_recommendation("nope").is_err());
}

#[test]
fn evaluation_expiry_adds_retention_days() {
    let evaluated_at = chrono::Utc::now();
    let expires = super::evaluation_expires_at(evaluated_at, 400);
    assert_eq!(expires, evaluated_at + chrono::Duration::days(400));
}

#[test]
fn combine_instantaneous_merges_max_and_sums_counters() {
    use nvbes_trust_risk::rules::FeatureMap;

    let mut features = FeatureMap::new();
    features.insert("events_1h".to_string(), 2.0);
    features.insert("network_risk_score_max_1h".to_string(), 40.0);
    features.insert("negative_labels".to_string(), 1.0);

    let mut instantaneous = FeatureMap::new();
    instantaneous.insert("events_1h".to_string(), 3.0);
    instantaneous.insert("network_risk_score_max_1h".to_string(), 70.0);
    instantaneous.insert("negative_labels".to_string(), 9.0);
    instantaneous.insert("positive_labels".to_string(), 4.0);

    super::combine_instantaneous(&mut features, instantaneous);
    assert_eq!(features["events_1h"], 5.0);
    assert_eq!(features["network_risk_score_max_1h"], 70.0);
    assert_eq!(features["negative_labels"], 1.0);
    assert_eq!(features["positive_labels"], 0.0);
}
