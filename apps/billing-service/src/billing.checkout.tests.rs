use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, header, method, path},
};

use crate::database::http_test_support::{
    bearer, bearer_jwt, test_router, test_router_with_config,
};
use crate::database::test_support::test_config;

#[sqlx::test(migrations = "./migrations")]
async fn checkout_requires_authentication(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let app = test_router(pool);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "plan_code": "standard_monthly" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn checkout_rejects_read_only_scope(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let principal = Uuid::new_v4();
    let config = crate::database::http_test_support::jwt_config();
    let app = test_router_with_config(pool, config);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("authorization", bearer_jwt(principal, "billing:read"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "plan_code": "standard_monthly" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn checkout_rejects_unknown_plan(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let app = test_router(pool);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("authorization", bearer(Uuid::new_v4()))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "plan_code": "missing_plan" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn checkout_rejects_empty_idempotency_key(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let app = test_router(pool);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("authorization", bearer(Uuid::new_v4()))
                .header("idempotency-key", "")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "plan_code": "standard_monthly" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn checkout_creates_mock_session_and_reuses_idempotency(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let principal = Uuid::new_v4();
    let app = test_router(pool.clone());
    let body = json!({ "plan_code": "standard_monthly", "account_type": "team" }).to_string();

    let first = app
        .clone()
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("authorization", bearer(principal))
                .header("idempotency-key", "idem-checkout-1")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(first.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        first_json["url"]
            .as_str()
            .unwrap()
            .contains("mock-checkout")
    );

    let second = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("authorization", bearer(principal))
                .header("idempotency-key", "idem-checkout-1")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(second.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(first_json["session_id"], second_json["session_id"]);

    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_checkout_sessions WHERE account_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn checkout_calls_stripe_api_when_not_dummy_key(pool: PgPool) {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": "cus_wiremock" })))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions"))
        .and(header("authorization", "Bearer sk_test_from_wiremock"))
        .and(body_string_contains("price_test_standard_monthly"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cs_test_wiremock",
            "url": "https://checkout.stripe.test/session"
        })))
        .mount(&mock)
        .await;

    let workspace = Uuid::new_v4();
    let mut config = test_config();
    config.stripe_secret_key = "sk_test_from_wiremock".into();
    config.stripe_api_base_url = mock.uri();

    let app = test_router_with_config(pool, config);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/checkout"))
                .header("authorization", bearer(Uuid::new_v4()))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "plan_code": "standard_monthly" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["session_id"], "cs_test_wiremock");
    assert_eq!(payload["url"], "https://checkout.stripe.test/session");
}
