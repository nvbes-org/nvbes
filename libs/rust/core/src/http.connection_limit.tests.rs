use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use tower::ServiceExt;

use super::{PerIpConcurrencyLayer, extract_client_ip, try_acquire};

#[test]
fn extract_client_ip_prefers_trusted_header() {
    let mut request = Request::builder()
        .header("x-nvbes-client-ip", "203.0.113.10")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        "127.0.0.1".parse::<IpAddr>().unwrap(),
        8080,
    )));
    assert_eq!(
        extract_client_ip(&request),
        Some("203.0.113.10".parse().unwrap())
    );
}

#[test]
fn extract_client_ip_falls_back_to_connect_info() {
    let mut request = Request::builder().body(Body::empty()).unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        "127.0.0.1".parse().unwrap(),
        8080,
    )));
    assert_eq!(
        extract_client_ip(&request),
        Some("127.0.0.1".parse().unwrap())
    );
}

#[test]
fn try_acquire_enforces_per_ip_limit() {
    let counter = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let ip: IpAddr = "203.0.113.10".parse().unwrap();
    let first = try_acquire(&counter, ip, 1);
    assert!(first.is_some());
    assert!(try_acquire(&counter, ip, 1).is_none());
    drop(first);
    assert!(try_acquire(&counter, ip, 1).is_some());
}

#[tokio::test]
async fn per_ip_concurrency_layer_returns_429_when_limit_exceeded() {
    let app = Router::new()
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(80)).await;
                "ok"
            }),
        )
        .layer(PerIpConcurrencyLayer::new(1));

    let build = |ip: &str| {
        Request::builder()
            .uri("/slow")
            .header("x-nvbes-client-ip", ip)
            .body(Body::empty())
            .expect("request")
    };

    let first = app.clone().oneshot(build("203.0.113.20"));
    let second = app.clone().oneshot(build("203.0.113.20"));
    let (first_response, second_response) = tokio::join!(first, second);
    let first_response = first_response.expect("first");
    let second_response = second_response.expect("second");
    let statuses = [first_response.status(), second_response.status()];
    assert!(
        statuses.contains(&StatusCode::OK) && statuses.contains(&StatusCode::TOO_MANY_REQUESTS),
        "expected one success and one 429, got {statuses:?}"
    );
}

#[test]
fn per_ip_concurrency_layer_constructs_with_limit() {
    let _layer = PerIpConcurrencyLayer::new(4);
}

#[tokio::test]
async fn per_ip_concurrency_allows_requests_without_client_ip() {
    let app = Router::new()
        .route("/ok", get(|| async { "ok" }))
        .layer(PerIpConcurrencyLayer::new(1));

    let first = app
        .clone()
        .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .expect("first");
    let second = app
        .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .expect("second");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
}
