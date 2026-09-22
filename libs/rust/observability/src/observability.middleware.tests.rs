use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
};
use tower::ServiceExt;

use crate::metrics::HttpMetrics;
use crate::request_id::REQUEST_ID_HEADER;
use crate::trace_context::{TRACEPARENT_HEADER, TRACESTATE_HEADER};

use super::{observe_request, request_path_template, server_timing_value};

#[test]
fn request_path_template_uses_matched_path_when_present() {
    assert_eq!(
        request_path_template(Some("/files/{file_id}")),
        "/files/{file_id}"
    );
}

#[test]
fn request_path_template_falls_back_when_unmatched() {
    assert_eq!(request_path_template(None), "unmatched");
}

#[test]
fn server_timing_reports_total_app_duration() {
    assert_eq!(server_timing_value(42), "app;dur=42");
    assert_eq!(server_timing_value(0), "app;dur=0");
}

fn app_with_status(status: StatusCode) -> Router {
    let metrics = HttpMetrics::new();
    Router::new()
        .route("/probe", get(move || async move { status }))
        .layer(middleware::from_fn_with_state(metrics, observe_request))
}

#[tokio::test]
async fn observe_request_injects_request_id_trace_and_server_timing_on_success() {
    let response = app_with_status(StatusCode::OK)
        .oneshot(
            Request::builder()
                .uri("/probe")
                .header(REQUEST_ID_HEADER, "req_middleware_ok")
                .header(
                    TRACEPARENT_HEADER,
                    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
                )
                .header(TRACESTATE_HEADER, "vendor=value")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(REQUEST_ID_HEADER).unwrap(),
        "req_middleware_ok"
    );
    assert!(response.headers().contains_key(TRACEPARENT_HEADER));
    assert_eq!(
        response.headers().get(TRACESTATE_HEADER).unwrap(),
        "vendor=value"
    );
    assert!(
        response
            .headers()
            .get("server-timing")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("app;dur=")
    );
}

#[tokio::test]
async fn observe_request_generates_trace_context_when_absent() {
    let response = app_with_status(StatusCode::NO_CONTENT)
        .oneshot(
            Request::builder()
                .uri("/probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    assert!(response.headers().contains_key(TRACEPARENT_HEADER));
}

#[tokio::test]
async fn observe_request_logs_client_and_server_error_branches() {
    for status in [StatusCode::BAD_REQUEST, StatusCode::INTERNAL_SERVER_ERROR] {
        let response = app_with_status(status)
            .oneshot(
                Request::builder()
                    .uri("/probe")
                    .header(REQUEST_ID_HEADER, "req_err_branch")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER).unwrap(),
            "req_err_branch"
        );
    }
}

#[tokio::test]
async fn observe_request_rejects_unsafe_inbound_request_id() {
    let response = app_with_status(StatusCode::OK)
        .oneshot(
            Request::builder()
                .uri("/probe")
                .header(REQUEST_ID_HEADER, "not safe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let returned = response
        .headers()
        .get(REQUEST_ID_HEADER)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(returned.starts_with("req_"));
    assert_ne!(returned, "not safe");
}
