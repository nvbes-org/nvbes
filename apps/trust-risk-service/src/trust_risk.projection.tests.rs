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
