use std::net::{IpAddr, SocketAddr};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::Request;

use super::{extract_client_ip, try_acquire};

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
