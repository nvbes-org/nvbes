use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::IntoResponse,
};
use chrono::NaiveDate;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    cockpit_auth::{DEFAULT_OPERATOR_ROLE, OperatorAuthPolicy},
    cockpit_model::ServiceId,
    cockpit_server::{PlatformCockpitState, create_platform_cockpit_router},
    operations_context::ContextClient,
    operations_error::OperationsError,
    operations_model::{Action, CaseCategory, Command},
    operations_service::execute,
};

fn token() -> String {
    let claims = json!({
        "sub":"operator-routes","role":DEFAULT_OPERATOR_ROLE,"amr":["mfa"],
        "auth_time": chrono::Utc::now().timestamp(),
        "iss":"test-identity","aud":"platform-operations",
        "exp": chrono::Utc::now().timestamp()+60,
        "nbf": chrono::Utc::now().timestamp()-1
    });
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(b"test-signing-key-for-platform-operations"),
    )
    .unwrap()
}

fn state(pool: PgPool) -> PlatformCockpitState {
    PlatformCockpitState {
        environment: "test".into(),
        auth_policy: Arc::new(OperatorAuthPolicy::test_policy()),
        db: pool,
        context: Arc::new(ContextClient::new(vec![], false).unwrap()),
    }
}

fn auth_request(method: &str, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

#[test]
fn operations_error_maps_to_http_statuses() {
    assert_eq!(
        OperationsError::Invalid("bad").into_response().status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        OperationsError::NotFound.into_response().status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        OperationsError::Conflict.into_response().status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        OperationsError::Forbidden.into_response().status(),
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn routes_cover_command_cases_audits_and_costs(pool: PgPool) {
    let actor = crate::cockpit_auth::OperatorSession {
        operator_id: "operator-routes".into(),
        role: DEFAULT_OPERATOR_ROLE.into(),
        has_mfa_step_up: true,
    };
    let open = Command {
        idempotency_key: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        reason: "Synthetic support verification".into(),
        action: Action::OpenCase {
            category: CaseCategory::Support,
            owner: ServiceId::Account,
            subject_id: Uuid::new_v4(),
            source: "synthetic ticket".into(),
            summary: "Route coverage case".into(),
            related_case_id: None,
        },
    };
    let created = execute(&pool, &actor, open.clone()).await.unwrap();
    let case_id = created.case_id.unwrap();

    execute(
        &pool,
        &actor,
        Command {
            idempotency_key: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            reason: "Recorded cost for FinOps review".into(),
            action: Action::RecordCost {
                month: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                provider: "Scaleway".into(),
                category: "compute".into(),
                actual_cents: 400,
                forecast_cents: 500,
                evidence: "synthetic invoice reference".into(),
                replaces: None,
            },
        },
    )
    .await
    .unwrap();

    let app = create_platform_cockpit_router(state(pool));

    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/cases")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let command_response = app
        .clone()
        .oneshot(auth_request(
            "POST",
            "/api/v1/commands",
            Body::from(serde_json::to_vec(&open).unwrap()),
        ))
        .await
        .unwrap();
    assert_eq!(command_response.status(), StatusCode::OK);

    let cases = app
        .clone()
        .oneshot(auth_request("GET", "/api/v1/cases", Body::empty()))
        .await
        .unwrap();
    assert_eq!(cases.status(), StatusCode::OK);
    let cases_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(cases.into_body(), 64_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(!cases_body["items"].as_array().unwrap().is_empty());

    let case_ctx = app
        .clone()
        .oneshot(auth_request(
            "GET",
            &format!("/api/v1/cases/{case_id}"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(case_ctx.status(), StatusCode::OK);

    let audits = app
        .clone()
        .oneshot(auth_request(
            "GET",
            &format!("/api/v1/audits?case_id={case_id}"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(audits.status(), StatusCode::OK);

    let costs = app
        .oneshot(auth_request(
            "GET",
            "/api/v1/costs?month=2026-09-01",
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(costs.status(), StatusCode::OK);
    let costs_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(costs.into_body(), 64_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(costs_body["review"], "within_recorded_budget");
    assert!(costs_body["actual_cents"].as_i64().unwrap() >= 400);
}

#[sqlx::test(migrations = "./migrations")]
async fn costs_review_covers_budget_decision_branches(pool: PgPool) {
    let actor = crate::cockpit_auth::OperatorSession {
        operator_id: "operator-routes".into(),
        role: DEFAULT_OPERATOR_ROLE.into(),
        has_mfa_step_up: true,
    };
    let app = create_platform_cockpit_router(state(pool.clone()));

    let unknown = app
        .clone()
        .oneshot(auth_request(
            "GET",
            "/api/v1/costs?month=2026-01-01",
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::OK);
    let unknown_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(unknown.into_body(), 64_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(unknown_body["review"], "unknown");

    async fn record(pool: &PgPool, actor: &crate::cockpit_auth::OperatorSession, forecast: u32) {
        execute(
            pool,
            actor,
            Command {
                idempotency_key: Uuid::new_v4(),
                correlation_id: Uuid::new_v4(),
                reason: "Recorded cost for FinOps review".into(),
                action: Action::RecordCost {
                    month: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
                    provider: format!("provider-{forecast}"),
                    category: "compute".into(),
                    actual_cents: forecast,
                    forecast_cents: forecast,
                    evidence: "synthetic invoice reference".into(),
                    replaces: None,
                },
            },
        )
        .await
        .unwrap();
    }

    record(&pool, &actor, 2_500).await;
    let economy = app
        .clone()
        .oneshot(auth_request(
            "GET",
            "/api/v1/costs?month=2026-02-01",
            Body::empty(),
        ))
        .await
        .unwrap();
    let economy_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(economy.into_body(), 64_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(economy_body["review"], "economy");

    record(&pool, &actor, 300).await;
    let critical = app
        .clone()
        .oneshot(auth_request(
            "GET",
            "/api/v1/costs?month=2026-02-01",
            Body::empty(),
        ))
        .await
        .unwrap();
    let critical_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(critical.into_body(), 64_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(critical_body["review"], "critical");

    record(&pool, &actor, 200).await;
    let stop = app
        .oneshot(auth_request(
            "GET",
            "/api/v1/costs?month=2026-02-01",
            Body::empty(),
        ))
        .await
        .unwrap();
    let stop_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(stop.into_body(), 64_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(stop_body["review"], "stop");
}
