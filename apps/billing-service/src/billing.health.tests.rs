use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::database::test_support::test_config;

#[sqlx::test(migrations = "./migrations")]
async fn live_and_ready_endpoints(pool: PgPool) {
    let config = test_config();
    let state = crate::app::BillingState {
        db: pool,
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };
    let app = crate::app::create_router(state);

    let live = app
        .clone()
        .oneshot(Request::get("/health/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(live.status(), StatusCode::OK);

    let ready = app
        .oneshot(Request::get("/health/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(ready.status(), StatusCode::OK);
}
