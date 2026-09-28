use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::database::http_test_support::test_router;

#[sqlx::test(migrations = "./migrations")]
async fn lists_active_plans_sorted_by_price(pool: PgPool) {
    let app = test_router(pool);
    let response = app
        .oneshot(Request::get("/billing/plans").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let plans: Vec<serde_json::Value> = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(plans.len() >= 3);
    assert_eq!(plans[0]["plan_code"], "free");
    assert!(
        plans[0]["amount_cents"].as_i64().unwrap() <= plans[1]["amount_cents"].as_i64().unwrap()
    );
}
