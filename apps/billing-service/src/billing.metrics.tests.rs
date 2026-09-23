use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::database::{connect_lazy, test_support::test_config};

#[tokio::test]
async fn metrics_requires_bearer_token() {
    let config = test_config();
    let db = connect_lazy(
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test",
        1,
    )
    .expect("lazy pool");
    let state = crate::app::BillingState {
        db,
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };
    let app = crate::app::create_router(state);

    let unauthorized = app
        .clone()
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let authorized = app
        .oneshot(
            Request::get("/metrics")
                .header("authorization", "Bearer metrics_fixture")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_rejects_when_token_is_not_configured() {
    let mut config = test_config();
    config.metrics_token = None;
    let db = connect_lazy(
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test",
        1,
    )
    .expect("lazy pool");
    let state = crate::app::BillingState {
        db,
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };
    let app = crate::app::create_router(state);
    let response = app
        .oneshot(
            Request::get("/metrics")
                .header("authorization", "Bearer anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
