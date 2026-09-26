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

#[sqlx::test(migrations = "./migrations")]
async fn transition_review_lists_and_rule_rollback_paths(pool: sqlx::PgPool) {
    let evaluation_id = seed_evaluation(&pool).await;
    let review_id = Uuid::new_v4();
    sqlx::query(
        r#"
            INSERT INTO trust_risk_review_cases (id, evaluation_id, state, expires_at)
            VALUES ($1, $2, 'open', clock_timestamp() + INTERVAL '400 days')
            "#,
    )
    .bind(review_id)
    .bind(evaluation_id)
    .execute(&pool)
    .await
    .expect("review");

    let service = OperationsService::new(grpc_test_support::state(pool.clone()));

    let listed = service
        .list_review_cases(operator_request(pb::ListReviewCasesRequest {
            state: pb::ReviewCaseState::Open as i32,
            limit: 10,
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("list open");
    assert_eq!(listed.get_ref().cases.len(), 1);

    let transitioned = service
        .transition_review_case(operator_request(pb::TransitionReviewCaseRequest {
            review_case_id: review_id.to_string(),
            target_state: pb::ReviewCaseState::InReview as i32,
            assign_to: Some("development-operator".into()),
            resolution_label: None,
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("transition");
    assert_eq!(
        transitioned.get_ref().state,
        i32::from(pb::ReviewCaseState::InReview)
    );

    let mismatched = service
        .transition_review_case(operator_request(pb::TransitionReviewCaseRequest {
            review_case_id: review_id.to_string(),
            target_state: pb::ReviewCaseState::Inconclusive as i32,
            assign_to: None,
            resolution_label: Some(pb::RiskLabel {
                label_id: Uuid::new_v4().to_string(),
                schema_version: 1,
                producer: "billing-checkout-fixture".to_string(),
                evaluation_id: evaluation_id.to_string(),
                review_case_id: Some(Uuid::new_v4().to_string()),
                kind: pb::RiskLabelKind::ConfirmedFraud.into(),
                source_class: pb::LabelSourceClass::Human.into(),
                source_id: "review:mismatch".to_string(),
                confidence: 0.9,
                actor: Some("other-actor".to_string()),
                knowledge_at: Some(prost_types::Timestamp {
                    seconds: chrono::Utc::now().timestamp(),
                    nanos: 0,
                }),
                evidence_reference: Some("case:mismatch".to_string()),
                mapping_version: "human-review-v1".to_string(),
                corrects_label_id: None,
            }),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect_err("label mismatch");
    assert_eq!(mismatched.code(), Code::InvalidArgument);

    let invalid_operator = service
        .stage_rule_set(Request::new(pb::StageRuleSetRequest {
            version: "x".into(),
            canonical_json: vec![],
            operator: Some(pb::OperatorContext {
                caller: "ab".into(),
                actor: "development-operator".into(),
                reason: "ok".into(),
                request_context: None,
            }),
        }))
        .await
        .expect_err("invalid operator");
    assert_eq!(invalid_operator.code(), Code::InvalidArgument);

    let oversized = service
        .stage_rule_set(operator_request(pb::StageRuleSetRequest {
            version: "too-big".into(),
            canonical_json: vec![0_u8; 256 * 1024 + 1],
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect_err("oversized");
    assert_eq!(oversized.code(), Code::ResourceExhausted);

    let at_budget = service
        .stage_rule_set(operator_request(pb::StageRuleSetRequest {
            version: "at-budget".into(),
            canonical_json: vec![0_u8; 256 * 1024],
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await;
    assert!(
        at_budget
            .as_ref()
            .err()
            .map(|status| status.code() != Code::ResourceExhausted)
            .unwrap_or(true),
        "exact budget must remain accepted (`>` not `>=`)"
    );

    let v1 = br#"{
            "version":"ops-rollback-v1","feature_version":"features-v1",
            "thresholds":{"challenge":30,"review":60,"deny":85},
            "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":30,"reasons":["velocity.high"],"minimum_recommendation":null}]
        }"#;
    let v2 = br#"{
            "version":"ops-rollback-v2","feature_version":"features-v1",
            "thresholds":{"challenge":30,"review":60,"deny":85},
            "rules":[{"code":"velocity","predicate":{"op":"gte","feature":"events_1h","value":20},"score_delta":40,"reasons":["velocity.high"],"minimum_recommendation":null}]
        }"#;
    service
        .stage_rule_set(operator_request(pb::StageRuleSetRequest {
            version: "ops-rollback-v1".into(),
            canonical_json: v1.to_vec(),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("stage v1");

    // Dual-control: seed activation through SQL after retiring the baseline
    // active rule set inserted by migrations.
    sqlx::query("UPDATE trust_risk_rule_sets SET state = 'retired' WHERE state = 'active'")
        .execute(&pool)
        .await
        .expect("retire baseline");
    sqlx::query(
            "UPDATE trust_risk_rule_sets SET state = 'active', activated_by = 'seed-activator', activated_at = clock_timestamp() WHERE version = 'ops-rollback-v1'",
        )
        .execute(&pool)
        .await
        .expect("seed active");

    service
        .stage_rule_set(operator_request(pb::StageRuleSetRequest {
            version: "ops-rollback-v2".into(),
            canonical_json: v2.to_vec(),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("stage v2");
    sqlx::query(
            "UPDATE trust_risk_rule_sets SET state = 'retired', created_by = 'other-stager' WHERE version = 'ops-rollback-v1'",
        )
        .execute(&pool)
        .await
        .expect("retire v1");
    sqlx::query(
            "UPDATE trust_risk_rule_sets SET state = 'active', activated_by = 'seed-activator', activated_at = clock_timestamp() WHERE version = 'ops-rollback-v2'",
        )
        .execute(&pool)
        .await
        .expect("activate v2");

    let rolled = service
        .rollback_rule_set(operator_request(pb::RollbackRuleSetRequest {
            version: "ops-rollback-v1".into(),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("rollback");
    assert_eq!(rolled.get_ref().version, "ops-rollback-v1");
    assert_eq!(rolled.get_ref().state, "active");
}

#[sqlx::test(migrations = "./migrations")]
async fn get_evaluation_includes_canonical_label_name(pool: sqlx::PgPool) {
    let evaluation_id = seed_evaluation(&pool).await;
    let label_id = Uuid::new_v4();
    sqlx::query(
        r#"
            INSERT INTO trust_risk_labels (
                id, schema_version, producer, evaluation_id, kind, source_class,
                source_id, confidence, knowledge_at, mapping_version, fingerprint, expires_at
            ) VALUES (
                $1, 1, 'billing-checkout-fixture', $2, 2, 1, 'review:canonical',
                0.9, clock_timestamp(), 'human-review-v1', $3,
                clock_timestamp() + INTERVAL '400 days'
            )
            "#,
    )
    .bind(label_id)
    .bind(evaluation_id)
    .bind(vec![3_u8; 32])
    .execute(&pool)
    .await
    .expect("label");
    sqlx::query(
        "INSERT INTO trust_risk_canonical_labels (evaluation_id, label_id) VALUES ($1, $2)",
    )
    .bind(evaluation_id)
    .bind(label_id)
    .execute(&pool)
    .await
    .expect("canonical");

    let service = OperationsService::new(grpc_test_support::state(pool));
    let response = service
        .get_evaluation(operator_request(pb::GetEvaluationRequest {
            evaluation_id: evaluation_id.to_string(),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("detail");
    assert_eq!(
        response.get_ref().canonical_label.as_deref(),
        Some("confirmed_fraud")
    );
}
