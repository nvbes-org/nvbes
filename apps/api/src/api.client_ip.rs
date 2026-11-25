//! Resolves the caller's address once, at the edge of the router.
//!
//! Identity throttles by `ConnectInfo`, which behind a managed reverse proxy is
//! the proxy itself. When the transport peer is a trusted proxy, the nearest
//! untrusted `X-Forwarded-For` hop replaces it; from any other peer forwarded
//! headers are stripped and the peer address is kept.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use nvbes_core::http::client_ip::set_trusted_client_ip;

#[derive(Clone)]
pub struct TrustedProxies(Arc<[String]>);

impl TrustedProxies {
    pub fn new(cidrs: &[String]) -> Self {
        Self(cidrs.into())
    }
}

pub async fn resolve_client_ip(
    State(proxies): State<TrustedProxies>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer);
    let selected = set_trusted_client_ip(request.headers_mut(), peer, &proxies.0)
        .and_then(|ip| ip.parse::<IpAddr>().ok());
    if let (Some(peer), Some(ip)) = (peer, selected) {
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(ip, peer.port())));
    }
    next.run(request).await
}

#[cfg(test)]
#[path = "api.client_ip.tests.rs"]
mod tests;
