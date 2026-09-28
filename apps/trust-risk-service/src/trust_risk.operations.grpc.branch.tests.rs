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
    .bind(format!("ops-branch-{evaluation_id}"))
    .bind(vec![7_u8; 32])
    .execute(pool)
    .await
    .expect("evaluation");
    evaluation_id
}

fn operator_request<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    *request.metadata_mut() = grpc_test_support::operator_metadata();
    request
}

#[sqlx::test(migrations = "./migrations")]
async fn list_and_transition_reject_invalid_wire_values(pool: sqlx::PgPool) {
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

    let service = OperationsService::new(grpc_test_support::state(pool));

    let bad_list = service
        .list_review_cases(operator_request(pb::ListReviewCasesRequest {
            state: 999,
            limit: 10,
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect_err("invalid list state");
    assert_eq!(bad_list.code(), Code::InvalidArgument);

    let bad_target = service
        .transition_review_case(operator_request(pb::TransitionReviewCaseRequest {
            review_case_id: review_id.to_string(),
            target_state: 999,
            assign_to: None,
            resolution_label: None,
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect_err("invalid target");
    assert_eq!(bad_target.code(), Code::InvalidArgument);

    let mut invalid_label = pb::RiskLabel {
        label_id: "not-a-uuid".into(),
        schema_version: 1,
        producer: "billing-checkout-fixture".into(),
        evaluation_id: evaluation_id.to_string(),
        review_case_id: Some(review_id.to_string()),
        kind: pb::RiskLabelKind::ConfirmedFraud.into(),
        source_class: pb::LabelSourceClass::Human.into(),
        source_id: "review:bad".into(),
        confidence: 0.9,
        actor: Some("development-operator".into()),
        knowledge_at: Some(prost_types::Timestamp {
            seconds: chrono::Utc::now().timestamp(),
            nanos: 0,
        }),
        evidence_reference: Some("case:bad".into()),
        mapping_version: "human-review-v1".into(),
        corrects_label_id: None,
    };
    let bad_label = service
        .transition_review_case(operator_request(pb::TransitionReviewCaseRequest {
            review_case_id: review_id.to_string(),
            target_state: pb::ReviewCaseState::InReview as i32,
            assign_to: None,
            resolution_label: Some(invalid_label.clone()),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect_err("invalid label");
    assert_eq!(bad_label.code(), Code::InvalidArgument);

    invalid_label.label_id = Uuid::new_v4().to_string();
    invalid_label.actor = Some("other-operator".into());
    let actor_mismatch = service
        .transition_review_case(operator_request(pb::TransitionReviewCaseRequest {
            review_case_id: review_id.to_string(),
            target_state: pb::ReviewCaseState::InReview as i32,
            assign_to: None,
            resolution_label: Some(invalid_label),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect_err("actor mismatch");
    assert_eq!(actor_mismatch.code(), Code::InvalidArgument);

    let short_actor = service
        .list_review_cases(Request::new(pb::ListReviewCasesRequest {
            state: pb::ReviewCaseState::Unspecified as i32,
            limit: 10,
            operator: Some(pb::OperatorContext {
                caller: "backoffice".into(),
                actor: "ab".into(),
                reason: "coverage exercise".into(),
                request_context: None,
            }),
        }))
        .await
        .expect_err("short actor");
    assert_eq!(short_actor.code(), Code::InvalidArgument);

    let short_reason = service
        .list_review_cases(Request::new(pb::ListReviewCasesRequest {
            state: pb::ReviewCaseState::Unspecified as i32,
            limit: 10,
            operator: Some(pb::OperatorContext {
                caller: "backoffice".into(),
                actor: "development-operator".into(),
                reason: "x".into(),
                request_context: None,
            }),
        }))
        .await
        .expect_err("short reason");
    assert_eq!(short_reason.code(), Code::InvalidArgument);

    let long_reason = service
        .list_review_cases(Request::new(pb::ListReviewCasesRequest {
            state: pb::ReviewCaseState::Unspecified as i32,
            limit: 10,
            operator: Some(pb::OperatorContext {
                caller: "backoffice".into(),
                actor: "development-operator".into(),
                reason: "x".repeat(301),
                request_context: None,
            }),
        }))
        .await
        .expect_err("long reason");
    assert_eq!(long_reason.code(), Code::InvalidArgument);
}

#[sqlx::test(migrations = "./migrations")]
async fn transition_accepts_matching_resolution_label(pool: sqlx::PgPool) {
    let evaluation_id = seed_evaluation(&pool).await;
    let review_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO trust_risk_review_cases (id, evaluation_id, state, expires_at)
        VALUES ($1, $2, 'in_review', clock_timestamp() + INTERVAL '400 days')
        "#,
    )
    .bind(review_id)
    .bind(evaluation_id)
    .execute(&pool)
    .await
    .expect("review");

    let service = OperationsService::new(grpc_test_support::state(pool));
    let label = pb::RiskLabel {
        label_id: Uuid::new_v4().to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".into(),
        evaluation_id: evaluation_id.to_string(),
        review_case_id: Some(review_id.to_string()),
        kind: pb::RiskLabelKind::ConfirmedFraud.into(),
        source_class: pb::LabelSourceClass::Human.into(),
        source_id: "review:resolve-ok".into(),
        confidence: 0.95,
        actor: Some("development-operator".into()),
        knowledge_at: Some(prost_types::Timestamp {
            seconds: chrono::Utc::now().timestamp(),
            nanos: 0,
        }),
        evidence_reference: Some("case:resolve-ok".into()),
        mapping_version: "human-review-v1".into(),
        corrects_label_id: None,
    };
    let resolved = service
        .transition_review_case(operator_request(pb::TransitionReviewCaseRequest {
            review_case_id: review_id.to_string(),
            target_state: pb::ReviewCaseState::Resolved as i32,
            assign_to: None,
            resolution_label: Some(label),
            operator: Some(grpc_test_support::operator_context()),
        }))
        .await
        .expect("resolved with label");
    assert_eq!(
        resolved.get_ref().state,
        i32::from(pb::ReviewCaseState::Resolved)
    );
}
