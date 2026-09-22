use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use tower::ServiceExt;

use super::BillingError;

async fn status_of(error: BillingError) -> StatusCode {
    let response = error.into_response();
    response.status()
}

#[tokio::test]
async fn maps_error_variants_to_http_status() {
    assert_eq!(
        status_of(BillingError::Unauthorized).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of(BillingError::Forbidden).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status_of(BillingError::NotFound).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of(BillingError::Invalid("bad")).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(BillingError::LiveModeRejected("live")).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(BillingError::Stripe("down".into())).await,
        StatusCode::BAD_GATEWAY
    );
}

#[tokio::test]
async fn error_body_includes_stable_code() {
    let response = BillingError::Forbidden.into_response();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["code"], "insufficient_scope");
    assert!(json["message"].as_str().unwrap().contains("scope"));
}

#[tokio::test]
async fn database_error_maps_to_internal() {
    let app = axum::Router::new().route(
        "/",
        axum::routing::get(|| async {
            BillingError::Database(sqlx::Error::Protocol("boom".into())).into_response()
        }),
    );
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
