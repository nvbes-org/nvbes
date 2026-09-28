use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::test_support;

#[tokio::test]
async fn retention_trigger_returns_unavailable_when_database_is_closed() {
    let state = test_support::state(
        sqlx::postgres::PgPoolOptions::new()
            .min_connections(0)
            .max_connections(1)
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
    );
    state.db.close().await;
    let app = super::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/retention")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn retention_trigger_succeeds_on_empty_database(pool: sqlx::PgPool) {
    let app = super::router(test_support::state(pool));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/retention")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
}
