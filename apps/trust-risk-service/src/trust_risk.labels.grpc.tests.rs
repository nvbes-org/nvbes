#[cfg(feature = "database-tests")]
mod database {
    use nvbes_trust_risk::proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_label_service_server::TrustRiskLabelService,
    };
    use tonic::{Code, Request};
    use uuid::Uuid;

    use super::super::LabelService;
    use crate::grpc_test_support;

    fn label_wire(evaluation_id: Uuid, label_id: Uuid) -> pb::RiskLabel {
        pb::RiskLabel {
            label_id: label_id.to_string(),
            schema_version: 1,
            producer: "billing-checkout-fixture".to_string(),
            evaluation_id: evaluation_id.to_string(),
            review_case_id: None,
            kind: pb::RiskLabelKind::ConfirmedFraud.into(),
            source_class: pb::LabelSourceClass::Human.into(),
            source_id: "review:grpc-coverage".to_string(),
            confidence: 0.91,
            actor: Some("development-operator".to_string()),
            knowledge_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            evidence_reference: Some("case:grpc".to_string()),
            mapping_version: "human-review-v1".to_string(),
            corrects_label_id: None,
        }
    }

    async fn seed_evaluation(pool: &sqlx::PgPool) -> Uuid {
        let evaluation_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluations (
                id, producer, assessment_key, operation_class, request_fingerprint,
                score, band, recommendation, feature_version, rule_set_version, expires_at
            ) VALUES (
                $1, 'billing-checkout-fixture', $2, 'payment.checkout', $3,
                70, 'high', 'review', 'features-v1', 'baseline-v1',
                clock_timestamp() + INTERVAL '400 days'
            )
            "#,
        )
        .bind(evaluation_id)
        .bind(format!("grpc-label-{evaluation_id}"))
        .bind(vec![5_u8; 32])
        .execute(pool)
        .await
        .expect("evaluation");
        evaluation_id
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_labels_persists_human_labels(pool: sqlx::PgPool) {
        let evaluation_id = seed_evaluation(&pool).await;
        let service = LabelService::new(grpc_test_support::state(pool));
        let mut request = Request::new(pb::SubmitLabelsRequest {
            labels: vec![label_wire(evaluation_id, Uuid::new_v4())],
        });
        *request.metadata_mut() = grpc_test_support::operator_metadata();
        let response = service.submit_labels(request).await.expect("submit");
        assert_eq!(response.get_ref().receipts.len(), 1);
        assert!(!response.get_ref().receipts[0].duplicate);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_labels_rejects_invalid_batches_and_auth(pool: sqlx::PgPool) {
        let service = LabelService::new(grpc_test_support::state(pool));
        let empty = service
            .submit_labels(Request::new(pb::SubmitLabelsRequest { labels: vec![] }))
            .await
            .expect_err("empty batch");
        assert_eq!(empty.code(), Code::InvalidArgument);

        let mut missing_actor = label_wire(Uuid::new_v4(), Uuid::new_v4());
        missing_actor.actor = None;
        let mut invalid_human = Request::new(pb::SubmitLabelsRequest {
            labels: vec![missing_actor],
        });
        *invalid_human.metadata_mut() = grpc_test_support::operator_metadata();
        let human = service
            .submit_labels(invalid_human)
            .await
            .expect_err("human label requires actor");
        assert_eq!(human.code(), Code::InvalidArgument);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_labels_rejects_producers_without_label_permission(pool: sqlx::PgPool) {
        let evaluation_id = seed_evaluation(&pool).await;
        let mut config = grpc_test_support::config("postgres://trust-risk/test");
        config
            .producers
            .get_mut("billing-checkout-fixture")
            .expect("producer")
            .can_label = false;
        let state = crate::app::TrustRiskState::new(config, pool);
        let service = LabelService::new(state);
        let mut wire = label_wire(evaluation_id, Uuid::new_v4());
        wire.source_class = pb::LabelSourceClass::VerifiedProduct.into();
        wire.actor = None;
        let mut request = Request::new(pb::SubmitLabelsRequest { labels: vec![wire] });
        *request.metadata_mut() = grpc_test_support::producer_metadata();
        let denied = service
            .submit_labels(request)
            .await
            .expect_err("can_label false");
        assert_eq!(denied.code(), Code::PermissionDenied);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_labels_rejects_oversized_batches(pool: sqlx::PgPool) {
        let service = LabelService::new(grpc_test_support::state(pool));
        let labels = (0..129)
            .map(|index| {
                let mut wire = label_wire(Uuid::new_v4(), Uuid::new_v4());
                wire.source_id = format!("review:overflow-{index}");
                wire
            })
            .collect();
        let mut request = Request::new(pb::SubmitLabelsRequest { labels });
        *request.metadata_mut() = grpc_test_support::operator_metadata();
        let overflow = service
            .submit_labels(request)
            .await
            .expect_err("batch too large");
        assert_eq!(overflow.code(), Code::InvalidArgument);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn submit_labels_accepts_verified_product_labels(pool: sqlx::PgPool) {
        let evaluation_id = seed_evaluation(&pool).await;
        let service = LabelService::new(grpc_test_support::state(pool));
        let mut wire = label_wire(evaluation_id, Uuid::new_v4());
        wire.source_class = pb::LabelSourceClass::VerifiedProduct.into();
        wire.actor = None;
        let mut request = Request::new(pb::SubmitLabelsRequest { labels: vec![wire] });
        *request.metadata_mut() = grpc_test_support::producer_metadata();
        let response = service.submit_labels(request).await.expect("product label");
        assert_eq!(response.get_ref().receipts.len(), 1);
    }
}

#[cfg(not(feature = "database-tests"))]
#[test]
fn label_grpc_database_tests_require_database_tests_feature() {}
