use axum::{body::Body, http::Request};
use tower::ServiceExt;

use super::live;

#[tokio::test]
async fn liveness_is_shallow_and_does_not_need_state() {
    let payload = live().await;
    assert_eq!(payload.0.status, "alive");
    assert_eq!(payload.0.service, "nvbes-email-worker");

    let response = axum::Router::new()
        .route("/health/live", axum::routing::get(|| async { "alive" }))
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn readiness_is_shallow_and_does_not_wake_the_database(pool: sqlx::PgPool) {
    let state = crate::test_support::state(pool);
    state.db.close().await;
    let ready = super::ready().await;
    assert_eq!(ready.0, axum::http::StatusCode::OK);
    assert_eq!(ready.1.0.status, "ready");

    let response = super::router(state)
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
