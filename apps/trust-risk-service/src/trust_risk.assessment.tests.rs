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
