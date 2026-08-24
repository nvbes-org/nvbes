use std::collections::HashMap;

use nvbes_trust_risk::{proto::nvbes::trust_risk::v1 as pb, signal::RiskSignal};

use super::fingerprint;
#[cfg(feature = "database-tests")]
use super::{PersistSignalError, persist_signal};

fn wire() -> pb::RiskSignal {
    pb::RiskSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d102".to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:network:example".to_string(),
        subjects: vec![pb::SubjectReference {
            kind: pb::SubjectKind::Network.into(),
            namespace: "nvbes.network".to_string(),
            opaque_id: "network:018f7f2d-fc7d-7b7a".to_string(),
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        }],
        attributes: HashMap::from([(
            "risk_score".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::UnsignedValue(82)),
            },
        )]),
        context: None,
        scope: pb::DataScope::Regional.into(),
    }
}

#[test]
fn canonical_fingerprint_is_stable() {
    let first = RiskSignal::try_from(wire()).unwrap();
    let second = RiskSignal::try_from(wire()).unwrap();
    assert_eq!(fingerprint(&first), fingerprint(&second));
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn persistence_is_idempotent_and_conflict_safe(pool: sqlx::PgPool) {
    let first_wire = wire();
    let first = RiskSignal::try_from(first_wire.clone()).unwrap();
    let accepted = persist_signal(&pool, &first_wire, &first, 30)
        .await
        .unwrap();
    assert!(!accepted.duplicate);

    let duplicate = persist_signal(&pool, &first_wire, &first, 30)
        .await
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(accepted.accepted_at, duplicate.accepted_at);

    let mut changed_wire = first_wire;
    changed_wire.partition_key = "regional:eu-west:network:different".to_string();
    let changed = RiskSignal::try_from(changed_wire.clone()).unwrap();
    assert!(matches!(
        persist_signal(&pool, &changed_wire, &changed, 30)
            .await
            .unwrap_err(),
        PersistSignalError::Conflict
    ));
}
