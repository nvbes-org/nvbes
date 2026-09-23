use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use crate::database::http_test_support::{
    bearer, bearer_jwt, test_router, test_router_with_config,
};
use crate::database::test_support::test_config;

#[sqlx::test(migrations = "./migrations")]
async fn portal_requires_billing_read_scope(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let config = crate::database::http_test_support::jwt_config();
    let app = test_router_with_config(pool, config);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/portal"))
                .header("authorization", bearer_jwt(Uuid::new_v4(), "account:read"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn portal_creates_mock_session(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let app = test_router(pool);
    let response = app
        .oneshot(
            Request::post(format!("/workspaces/{workspace}/billing/portal"))
                .header("authorization", bearer(Uuid::new_v4()))
                .body(Body::empty())
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
    assert!(payload["url"].as_str().unwrap().contains("mock-portal"));
}

#[sqlx::test(migrations = "./migrations")]
async fn overview_defaults_to_free_without_subscription(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let app = test_router(pool);
    let response = app
        .oneshot(
            Request::get(format!("/workspaces/{workspace}/billing/overview"))
                .header("authorization", bearer(Uuid::new_v4()))
                .body(Body::empty())
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
    assert_eq!(payload["plan_code"], "free");
    assert_eq!(payload["status"], "active");
    assert_eq!(payload["account_id"], workspace.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn overview_returns_latest_subscription(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let customer_id = format!("cus_{}", Uuid::new_v4().simple());
    let sub_id = format!("sub_{}", Uuid::new_v4().simple());
    let period_end = Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
         VALUES ($1, 'team', $2, 'portal@t.test')",
    )
    .bind(workspace)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO billing_subscriptions (
            account_id, account_type, stripe_subscription_id, stripe_customer_id,
            plan_code, status, current_period_end, cancel_at_period_end
         ) VALUES ($1, 'team', $2, $3, 'pro_monthly', 'active', $4, true)",
    )
    .bind(workspace)
    .bind(&sub_id)
    .bind(&customer_id)
    .bind(period_end)
    .execute(&pool)
    .await
    .unwrap();

    let app = test_router(pool);
    let response = app
        .oneshot(
            Request::get(format!("/workspaces/{workspace}/billing/overview"))
                .header("authorization", bearer(Uuid::new_v4()))
                .body(Body::empty())
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
    assert_eq!(payload["plan_code"], "pro_monthly");
    assert_eq!(payload["customer_id"], customer_id);
    assert_eq!(payload["cancel_at_period_end"], true);
}

#[sqlx::test(migrations = "./migrations")]
async fn portal_calls_stripe_api_when_not_dummy_key(pool: PgPool) {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": "cus_wiremock" })))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/billing_portal/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "url": "https://billing.stripe.test/portal"
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
            Request::post(format!("/workspaces/{workspace}/billing/portal"))
                .header("authorization", bearer(Uuid::new_v4()))
                .body(Body::empty())
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
    assert_eq!(payload["url"], "https://billing.stripe.test/portal");
}
