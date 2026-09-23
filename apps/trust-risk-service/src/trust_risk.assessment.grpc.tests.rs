#[cfg(feature = "database-tests")]
mod database {
    use std::collections::HashMap;

    use nvbes_trust_risk::proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_assessment_service_server::TrustRiskAssessmentService,
    };
    use tonic::{Code, Request};

    use super::super::AssessmentService;
    use crate::grpc_test_support;

    fn assess_request() -> pb::AssessRiskRequest {
        let subject = pb::SubjectReference {
            kind: pb::SubjectKind::Principal.into(),
            namespace: "nvbes.identity".to_string(),
            opaque_id: "principal:018f7f2d-grpc".to_string(),
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        };
        let signal = pb::RiskSignal {
            signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d401".to_string(),
            schema_version: 1,
            producer: "billing-checkout-fixture".to_string(),
            signal_kind: "network.reputation".to_string(),
            occurred_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            partition_key: "regional:eu-west:principal:grpc".to_string(),
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
            assessment_key: "checkout:grpc-coverage".to_string(),
            operation_class: "payment.checkout".to_string(),
            subjects: vec![subject],
            instantaneous_signals: vec![signal],
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn assess_risk_persists_evaluation(pool: sqlx::PgPool) {
        let service = AssessmentService::new(grpc_test_support::state(pool));
        let mut request = Request::new(assess_request());
        *request.metadata_mut() = grpc_test_support::producer_metadata();
        let response = service.assess_risk(request).await.expect("assess");
        assert!(!response.get_ref().duplicate);
        assert!(!response.get_ref().evaluation_id.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn assess_risk_rejects_missing_auth_and_invalid_payload(pool: sqlx::PgPool) {
        let service = AssessmentService::new(grpc_test_support::state(pool));
        let unauthorized = service
            .assess_risk(Request::new(assess_request()))
            .await
            .expect_err("missing bearer");
        assert_eq!(unauthorized.code(), Code::Unauthenticated);

        let mut invalid = Request::new(assess_request());
        *invalid.metadata_mut() = grpc_test_support::producer_metadata();
        invalid.get_mut().assessment_key.clear();
        let bad = service
            .assess_risk(invalid)
            .await
            .expect_err("invalid assessment");
        assert_eq!(bad.code(), Code::InvalidArgument);
    }
}

#[cfg(not(feature = "database-tests"))]
#[test]
fn assessment_grpc_database_tests_require_database_tests_feature() {}
