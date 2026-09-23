use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use nvbes_trust_risk::{proto::nvbes::trust_risk::v1 as pb, signal::RiskSignal};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use uuid::Uuid;

use super::{MAX_PROJECTION_ATTEMPTS, derive_features, is_too_late, retry_delay};

fn signal(id: &str, occurred_at: i64, risk_score: u64) -> RiskSignal {
    RiskSignal::try_from(pb::RiskSignal {
        signal_id: id.to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: occurred_at,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:network:example".to_string(),
        subjects: vec![pb::SubjectReference {
            kind: pb::SubjectKind::Principal.into(),
            namespace: "nvbes.identity".to_string(),
            opaque_id: "principal:018f7f2d-fc7d".to_string(),
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        }],
        attributes: HashMap::from([(
            "risk_score".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::UnsignedValue(risk_score)),
            },
        )]),
        context: None,
        scope: pb::DataScope::Regional.into(),
    })
    .unwrap()
}

#[test]
fn event_time_windows_are_deterministic_and_idempotent() {
    let watermark = Utc.timestamp_opt(1_787_590_000, 0).unwrap();
    let signals = vec![
        signal(
            "018f7f2d-fc7d-7b7a-9f72-3abddda8d201",
            watermark.timestamp() - 30,
            90,
        ),
        signal(
            "018f7f2d-fc7d-7b7a-9f72-3abddda8d202",
            watermark.timestamp() - 7_200,
            20,
        ),
    ];
    let first = derive_features(&signals, watermark);
    let second = derive_features(&signals, watermark);
    assert_eq!(first, second);
    assert_eq!(first["events_1h"], 1.0);
    assert_eq!(first["events_24h"], 2.0);
    assert_eq!(first["high_risk_events_1h"], 1.0);
    assert_eq!(first["linked_principals_24h"], 1.0);
}

#[test]
fn lateness_retry_and_quarantine_are_bounded() {
    let watermark = Utc.timestamp_opt(1_787_590_000, 0).unwrap();
    assert!(!is_too_late(
        watermark - chrono::Duration::hours(24),
        watermark
    ));
    assert!(is_too_late(
        watermark - chrono::Duration::hours(25),
        watermark
    ));
    let id = Uuid::parse_str("018f7f2d-fc7d-7b7a-9f72-3abddda8d201").unwrap();
    assert!(retry_delay(1, id).is_some());
    assert!(retry_delay(MAX_PROJECTION_ATTEMPTS, id).is_none());
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 2048, rng_seed: RngSeed::Fixed(20260912), ..ProptestConfig::default()
    })]

    #[test]
    fn properties_projection_is_order_independent_and_respects_event_windows(
        events in prop::collection::vec((-1000_i64..100_000, 0_u64..=100), 0..32),
    ) {
        let watermark = Utc.timestamp_opt(1_787_590_000, 0).unwrap();
        let signals: Vec<_> = events.iter().enumerate().map(|(index, &(age, risk))| {
            signal(&Uuid::from_u128(index as u128 + 1).to_string(), watermark.timestamp() - age, risk)
        }).collect();
        let projected = derive_features(&signals, watermark);
        prop_assert_eq!(&projected, &derive_features(signals.iter().rev(), watermark));
        prop_assert_eq!(projected["events_1h"], events.iter().filter(|(age, _)| (0..=3600).contains(age)).count() as f64);
        prop_assert_eq!(projected["events_24h"], events.iter().filter(|(age, _)| (0..=86400).contains(age)).count() as f64);
        prop_assert_eq!(projected["high_risk_events_1h"], events.iter().filter(|(age, risk)| (0..=3600).contains(age) && *risk >= 75).count() as f64);
    }

    #[test]
    fn properties_retry_budget_is_bounded_and_repeatable(attempt in 0_u32..100, id in any::<u128>()) {
        let id = Uuid::from_u128(id);
        let delay = retry_delay(attempt, id);
        prop_assert_eq!(delay, retry_delay(attempt, id));
        prop_assert_eq!(delay.is_some(), attempt < MAX_PROJECTION_ATTEMPTS);
        if let Some(delay) = delay {
            prop_assert!((1..=60).contains(&delay.as_secs()));
        }
    }
}

#[test]
fn derive_features_skips_future_and_stale_events() {
    let watermark = Utc.timestamp_opt(1_787_590_000, 0).unwrap();
    let future = signal(
        "018f7f2d-fc7d-7b7a-9f72-3abddda8d401",
        watermark.timestamp() + 60,
        90,
    );
    let stale = signal(
        "018f7f2d-fc7d-7b7a-9f72-3abddda8d402",
        watermark.timestamp() - 90_000,
        90,
    );
    let features = derive_features([future, stale].iter().collect::<Vec<_>>(), watermark);
    assert_eq!(features["events_1h"], 0.0);
    assert_eq!(features["events_24h"], 0.0);
}

#[test]
fn derive_features_tracks_network_automation_and_tenant_subjects() {
    let watermark = Utc.timestamp_opt(1_787_590_000, 0).unwrap();
    let tenant_signal = RiskSignal::try_from(pb::RiskSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d301".to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: watermark.timestamp() - 100,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:tenant:example".to_string(),
        subjects: vec![pb::SubjectReference {
            kind: pb::SubjectKind::Tenant.into(),
            namespace: "nvbes.tenant".to_string(),
            opaque_id: "tenant:018f7f2d".to_string(),
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        }],
        attributes: HashMap::from([(
            "risk_score".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::UnsignedValue(80)),
            },
        )]),
        context: None,
        scope: pb::DataScope::Regional.into(),
    })
    .unwrap();
    let automation_signal = RiskSignal::try_from(pb::RiskSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d302".to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "automation.score".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: watermark.timestamp() - 50,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:automation:example".to_string(),
        subjects: vec![pb::SubjectReference {
            kind: pb::SubjectKind::Principal.into(),
            namespace: "nvbes.identity".to_string(),
            opaque_id: "principal:018f7f2d-auto".to_string(),
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        }],
        attributes: HashMap::from([(
            "confidence".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::DecimalValue(0.82)),
            },
        )]),
        context: None,
        scope: pb::DataScope::Regional.into(),
    })
    .unwrap();
    let features = derive_features(
        [tenant_signal, automation_signal]
            .iter()
            .collect::<Vec<_>>(),
        watermark,
    );
    assert_eq!(features["network_risk_score_max_1h"], 80.0);
    assert_eq!(features["linked_tenants_24h"], 1.0);
    assert_eq!(features["automation_confidence_max_1h"], 0.82);
}

#[cfg(feature = "database-tests")]
mod runtime {
    use std::time::Duration;

    use tokio::sync::watch;

    use super::super::run;
    use crate::grpc_test_support;

    #[sqlx::test(migrations = "./migrations")]
    async fn run_loop_updates_projection_heartbeat(pool: sqlx::PgPool) {
        let state = grpc_test_support::state(pool);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let task = tokio::spawn(run(state.clone(), shutdown_rx));
        tokio::time::sleep(Duration::from_millis(400)).await;
        shutdown_tx.send(true).unwrap();
        task.await.unwrap();
        assert!(state.projection_heartbeat.read().await.is_some());
    }
}
