use super::*;
use axum::{Router, body::Body, http::Request, routing::get};
use tower::ServiceExt;

async fn observed_peer(peer: &str, forwarded_for: Option<&str>, proxies: &[&str]) -> String {
    let proxies: Vec<String> = proxies.iter().map(|cidr| (*cidr).to_owned()).collect();
    let app = Router::new()
        .route(
            "/",
            get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.to_string() }),
        )
        .layer(axum::middleware::from_fn_with_state(
            TrustedProxies::new(&proxies),
            resolve_client_ip,
        ));
    let mut request = Request::get("/");
    if let Some(value) = forwarded_for {
        request = request.header("x-forwarded-for", value);
    }
    let mut request = request.body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 64)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn untrusted_peer_keeps_its_own_address() {
    assert_eq!(
        observed_peer("198.51.100.7:4000", Some("203.0.113.10"), &["10.0.0.0/8"]).await,
        "198.51.100.7:4000"
    );
}

#[tokio::test]
async fn trusted_proxy_is_replaced_by_the_nearest_untrusted_hop() {
    assert_eq!(
        observed_peer(
            "10.0.0.5:4000",
            Some("192.0.2.99, 203.0.113.10"),
            &["10.0.0.0/8"]
        )
        .await,
        "203.0.113.10:4000"
    );
}

#[tokio::test]
async fn trusted_proxy_without_forwarding_header_stays_the_peer() {
    assert_eq!(
        observed_peer("10.0.0.5:4000", None, &["10.0.0.0/8"]).await,
        "10.0.0.5:4000"
    );
}
