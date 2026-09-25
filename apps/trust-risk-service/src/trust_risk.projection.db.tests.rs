#[test]
fn projection_error_codes_are_stable() {
    use super::ProjectionError;

    assert_eq!(ProjectionError::InvalidPayload.code(), "invalid_payload");
    assert_ne!(ProjectionError::InvalidPayload.code(), "");
    assert_ne!(ProjectionError::InvalidPayload.code(), "xyzzy");
}

#[cfg(feature = "database-tests")]
mod database {
    use std::collections::HashMap;

    use nvbes_trust_risk::{proto::nvbes::trust_risk::v1 as pb, signal::RiskSignal};
    use uuid::Uuid;

    use super::super::{ProjectionError, process_batch};
    use crate::ingress_db::persist_signal;

    fn wire_signal(id: Uuid) -> pb::RiskSignal {
        pb::RiskSignal {
            signal_id: id.to_string(),
            schema_version: 1,
            producer: "billing-checkout-fixture".to_string(),
            signal_kind: "network.reputation".to_string(),
            occurred_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            partition_key: "regional:eu-west:principal:projection".to_string(),
            subjects: vec![pb::SubjectReference {
                kind: pb::SubjectKind::Principal.into(),
                namespace: "nvbes.identity".to_string(),
                opaque_id: "principal:018f7f2d-proj".to_string(),
                scope: pb::DataScope::Regional.into(),
                tenant_id: None,
            }],
            attributes: HashMap::from([(
                "risk_score".to_string(),
                pb::AttributeValue {
                    value: Some(pb::attribute_value::Value::UnsignedValue(90)),
                },
            )]),
            context: None,
            scope: pb::DataScope::Regional.into(),
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn process_batch_projects_claimed_signals(pool: sqlx::PgPool) {
        let id = Uuid::new_v4();
        let wire = wire_signal(id);
        let domain = RiskSignal::try_from(wire.clone()).expect("domain");
        persist_signal(&pool, &wire, &domain, 30)
            .await
            .expect("persist");

        let processed = process_batch(&pool, Uuid::new_v4(), 16)
            .await
            .expect("project");
        assert_eq!(processed, 1);

        let projected: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT projected_at FROM trust_risk_signals WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("projected_at");
        assert!(projected.is_some());

        let features: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM trust_risk_feature_state WHERE opaque_id = 'principal:018f7f2d-proj'",
        )
        .fetch_one(&pool)
        .await
        .expect("features");
        assert_eq!(features, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn process_batch_quarantines_invalid_payload_after_retries(pool: sqlx::PgPool) {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signals (
                id, schema_version, producer, signal_kind, occurred_at, partition_key,
                scope, fingerprint, payload, retention_deadline, projection_attempts,
                next_projection_at
            ) VALUES ($1, 1, 'billing-checkout-fixture', 'network.reputation', clock_timestamp(),
                      'regional:eu-west:principal:bad', 1, $2, $3,
                      clock_timestamp() + INTERVAL '30 days', 7, clock_timestamp())
            "#,
        )
        .bind(id)
        .bind(vec![1_u8; 32])
        .bind(b"not-a-protobuf".as_slice())
        .execute(&pool)
        .await
        .expect("seed invalid");

        let processed = process_batch(&pool, Uuid::new_v4(), 8)
            .await
            .expect("claim invalid");
        assert_eq!(processed, 1);

        let quarantined: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT quarantined_at FROM trust_risk_signals WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("quarantine");
        assert!(quarantined.is_some());

        let code: String = sqlx::query_scalar(
            "SELECT error_code FROM trust_risk_projection_quarantine WHERE signal_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("quarantine row");
        assert_eq!(code, ProjectionError::InvalidPayload.code());
    }
}
