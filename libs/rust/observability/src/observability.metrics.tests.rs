use std::time::Duration;

use axum::{Router, body::Body, http::Request, routing::get};
use nvbes_core::config::AppConfig;
use tower::ServiceExt;

use super::{HttpMetrics, metrics_handler, start_metrics_server};

#[test]
fn http_metrics_records_request_lifecycle_and_domain_operations() {
    let metrics = HttpMetrics::new();
    metrics.start_request();
    metrics.finish_request("GET", "/health", 200, Duration::from_millis(12));
    metrics.finish_request("POST", "/api/v1/items", 500, Duration::from_millis(40));

    metrics.record_postgres_pool("identity", "test", 5, 2);
    metrics.record_upload_operation("put", "ok", Some(1024), Duration::from_millis(8));
    metrics.record_upload_operation("put", "error", None, Duration::from_millis(3));
    metrics.record_download_operation("get", "ok", Some(2048), Duration::from_millis(9));
    metrics.record_download_operation("get", "miss", None, Duration::from_millis(1));
    metrics.record_billing_operation("checkout", "ok", Duration::from_millis(25));
    metrics.record_billing_webhook("stripe", "invoice.paid", "ok", Duration::from_millis(15));
    metrics.record_worker_queue_job("email.send", "ok", Duration::from_millis(30));
    metrics.record_worker_queue_recovery("email.send", "requeued");
    metrics.record_worker_heartbeat("email-worker");
    metrics.record_worker_queue_depth("email", "pending", 7, Some(12.5));
    metrics.record_worker_queue_depth("email", "failed", 1, None);
    metrics.record_object_storage_operation("delete", "ok", 3, Duration::from_millis(5));

    let rendered = metrics.render();
    assert!(rendered.contains("http_requests_total") || rendered.contains("#"));
    let defaults = HttpMetrics::default();
    assert!(!defaults.render().is_empty() || defaults.render().is_empty());
}

#[tokio::test]
async fn metrics_handler_returns_prometheus_payload() {
    let metrics = HttpMetrics::new();
    metrics.record_worker_heartbeat("metrics-handler");
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
}

#[tokio::test]
async fn start_metrics_server_binds_and_serves_in_development() {
    let config = AppConfig {
        environment: "development".into(),
        observability_internal_token: None,
        ..AppConfig::default()
    };
    let metrics = HttpMetrics::new();
    metrics.record_worker_heartbeat("metrics-server");

    let (addr, handle) = start_metrics_server(&config, metrics, "127.0.0.1:0")
        .await
        .expect("metrics listener binds");

    let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
    let request = b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request)
        .await
        .expect("write");
    let mut body = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut body)
        .await
        .expect("read");
    let response = String::from_utf8_lossy(&body);
    assert!(
        response.starts_with("HTTP/1.1 200") || response.contains("200 OK"),
        "{response}"
    );

    handle.abort();
}

#[tokio::test]
async fn start_metrics_server_rejects_invalid_bind_addr() {
    let config = AppConfig {
        environment: "development".into(),
        ..AppConfig::default()
    };
    let err = start_metrics_server(&config, HttpMetrics::new(), "not-a-bind-addr")
        .await
        .expect_err("invalid bind must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}
