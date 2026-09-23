use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;

use crate::operations_types::{band_value, label_kind_name, recommendation_value, review_state};

#[test]
fn operations_conversions_do_not_invent_values() {
    assert_eq!(band_value("corrupt"), pb::RiskBand::Unspecified as i32);
    assert_eq!(
        recommendation_value("corrupt"),
        pb::RiskRecommendation::Unspecified as i32
    );
    assert_eq!(label_kind_name(99), None);
    assert!(review_state(pb::ReviewCaseState::Unspecified).is_err());
}

#[cfg(feature = "database-tests")]
mod grpc {
    use nvbes_trust_risk::proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_operations_service_server::TrustRiskOperationsService,
    };
    use tonic::{Code, Request};
    use uuid::Uuid;

    use super::super::OperationsService;
    use crate::grpc_test_support;

    async fn seed_evaluation(pool: &sqlx::PgPool) -> Uuid {
        let evaluation_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluations (
                id, producer, assessment_key, operation_class, request_fingerprint,
                score, band, recommendation, feature_version, rule_set_version, expires_at
            ) VALUES (
                $1, 'billing-checkout-fixture', $2, 'payment.checkout', $3,
                80, 'critical', 'review', 'features-v1', 'baseline-v1',
                clock_timestamp() + INTERVAL '400 days'
            )
            "#,
        )
        .bind(evaluation_id)
        .bind(format!("ops-grpc-{evaluation_id}"))
        .bind(vec![7_u8; 32])
        .execute(pool)
        .await
        .expect("evaluation");
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluation_features (evaluation_id, name, value)
            VALUES ($1, 'events_1h', 2.0)
            "#,
        )
        .bind(evaluation_id)
        .execute(pool)
        .await
        .expect("feature");
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluation_reasons (evaluation_id, ordinal, code)
            VALUES ($1, 0, 'velocity.high')
            "#,
        )
        .bind(evaluation_id)
        .execute(pool)
        .await
        .expect("reason");
        evaluation_id
    }

    fn operator_request<T>(message: T) -> Request<T> {
        let mut request = Request::new(message);
        *request.metadata_mut() = grpc_test_support::operator_metadata();
        request
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_evaluation_returns_persisted_detail(pool: sqlx::PgPool) {
        let evaluation_id = seed_evaluation(&pool).await;
        let service = OperationsService::new(grpc_test_support::state(pool));
        let response = service
            .get_evaluation(operator_request(pb::GetEvaluationRequest {
                evaluation_id: evaluation_id.to_string(),
                operator: Some(grpc_test_support::operator_context()),
            }))
            .await
            .expect("detail");
        assert_eq!(
            response
                .get_ref()
                .evaluation
                .as_ref()
                .unwrap()
                .evaluation_id,
            evaluation_id.to_string()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn list_review_cases_and_rule_mutations_require_operator_context(pool: sqlx::PgPool) {
        let service = OperationsService::new(grpc_test_support::state(pool.clone()));
        let missing = service
            .list_review_cases(Request::new(pb::ListReviewCasesRequest {
                state: pb::ReviewCaseState::Unspecified as i32,
                limit: 10,
                operator: None,
            }))
            .await
            .expect_err("operator required");
        assert_eq!(missing.code(), Code::InvalidArgument);

        let listed = service
            .list_review_cases(operator_request(pb::ListReviewCasesRequest {
                state: pb::ReviewCaseState::Unspecified as i32,
                limit: 10,
                operator: Some(grpc_test_support::operator_context()),
            }))
            .await
            .expect("list");
        assert!(listed.get_ref().cases.is_empty());

        let json = br#"{
            "version":"candidate-grpc-v1","feature_version":"features-v1",
            "thresholds":{"challenge":30,"review":60,"deny":85},
            "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":30,"reasons":["velocity.high"],"minimum_recommendation":null}]
        }"#;
        let staged = service
            .stage_rule_set(operator_request(pb::StageRuleSetRequest {
                version: "candidate-grpc-v1".to_string(),
                canonical_json: json.to_vec(),
                operator: Some(grpc_test_support::operator_context()),
            }))
            .await
            .expect("stage");
        assert_eq!(staged.get_ref().version, "candidate-grpc-v1");

        let activated = service
            .activate_rule_set(operator_request(pb::ActivateRuleSetRequest {
                version: "candidate-grpc-v1".to_string(),
                operator: Some(grpc_test_support::operator_context()),
            }))
            .await
            .expect_err("dual control");
        assert_eq!(activated.code(), Code::FailedPrecondition);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_evaluation_returns_not_found_for_unknown_id(pool: sqlx::PgPool) {
        let service = OperationsService::new(grpc_test_support::state(pool));
        let missing = service
            .get_evaluation(operator_request(pb::GetEvaluationRequest {
                evaluation_id: Uuid::new_v4().to_string(),
                operator: Some(grpc_test_support::operator_context()),
            }))
            .await
            .expect_err("missing evaluation");
        assert_eq!(missing.code(), Code::NotFound);
    }
}
