#[cfg(feature = "database-tests")]
mod database {
    use std::collections::HashMap;

    use nvbes_trust_risk::proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_signal_service_server::TrustRiskSignalService,
    };
    use tonic::{Code, Request};

    use super::super::SignalService;
    use crate::grpc_test_support;

    fn signal_wire() -> pb::RiskSignal {
        pb::RiskSignal {
            signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d501".to_string(),
            schema_version: 1,
            producer: "billing-checkout-fixture".to_string(),
            signal_kind: "network.reputation".to_string(),
            occurred_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            partition_key: "regional:eu-west:network:grpc".to_string(),
            subjects: vec![pb::SubjectReference {
                kind: pb::SubjectKind::Network.into(),
                namespace: "nvbes.network".to_string(),
                opaque_id: "network:018f7f2d-grpc".to_string(),
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

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_signals_persists_and_replays(pool: sqlx::PgPool) {
        let service = SignalService::new(grpc_test_support::state(pool));
        let mut request = Request::new(pb::SubmitSignalsRequest {
            signals: vec![signal_wire()],
        });
        *request.metadata_mut() = grpc_test_support::producer_metadata();
        let first = service.submit_signals(request).await.expect("submit");
        assert!(!first.get_ref().receipts[0].duplicate);

        let mut replay = Request::new(pb::SubmitSignalsRequest {
            signals: vec![signal_wire()],
        });
        *replay.metadata_mut() = grpc_test_support::producer_metadata();
        let second = service.submit_signals(replay).await.expect("replay");
        assert!(second.get_ref().receipts[0].duplicate);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_signals_rejects_invalid_batches_and_families(pool: sqlx::PgPool) {
        let service = SignalService::new(grpc_test_support::state(pool));
        let empty = service
            .submit_signals(Request::new(pb::SubmitSignalsRequest { signals: vec![] }))
            .await
            .expect_err("empty");
        assert_eq!(empty.code(), Code::InvalidArgument);

        let mut denied = signal_wire();
        denied.signal_kind = "identity.login".to_string();
        let mut request = Request::new(pb::SubmitSignalsRequest {
            signals: vec![denied],
        });
        *request.metadata_mut() = grpc_test_support::producer_metadata();
        let err = service.submit_signals(request).await.expect_err("prefix");
        assert_eq!(err.code(), Code::PermissionDenied);
    }
}

#[cfg(not(feature = "database-tests"))]
#[test]
fn ingress_grpc_database_tests_require_database_tests_feature() {}
