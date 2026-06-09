use std::net::{IpAddr, SocketAddr};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use ipnet::IpNet;

use crate::config::AppConfig;

pub const TRUSTED_CLIENT_IP_HEADER: &str = "x-nvbes-client-ip";

pub async fn trusted_client_ip_middleware(
    State(config): State<AppConfig>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let peer_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| *addr);

    set_trusted_client_ip(
        request.headers_mut(),
        peer_addr,
        &config.trusted_proxy_cidrs,
    );

    next.run(request).await
}

pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get(TRUSTED_CLIENT_IP_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn set_trusted_client_ip(
    headers: &mut HeaderMap,
    peer_addr: Option<SocketAddr>,
    trusted_proxy_cidrs: &[String],
) -> Option<String> {
    let forwarded_for = header_value(headers, "x-forwarded-for");
    let real_ip = header_value(headers, "x-real-ip");

    headers.remove("x-forwarded-for");
    headers.remove("x-real-ip");
    headers.remove("cf-connecting-ip");
    headers.remove(TRUSTED_CLIENT_IP_HEADER);

    let peer_ip = peer_addr.map(|addr| addr.ip());
    let proxy_is_trusted = peer_ip
        .map(|ip| is_trusted_proxy(ip, trusted_proxy_cidrs))
        .unwrap_or(false);

    let selected = if proxy_is_trusted {
        forwarded_for
            .as_deref()
            .and_then(first_forwarded_ip)
            .or_else(|| real_ip.as_deref().and_then(parse_ip))
            .or(peer_ip)
    } else {
        peer_ip
    };

    let selected = selected.map(|ip| ip.to_string());
    if let Some(ip) = &selected
        && let Ok(value) = HeaderValue::from_str(ip)
    {
        headers.insert(TRUSTED_CLIENT_IP_HEADER, value);
    }

    selected
}

fn header_value(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn first_forwarded_ip(value: &str) -> Option<IpAddr> {
    value.split(',').find_map(parse_ip)
}

fn parse_ip(value: &str) -> Option<IpAddr> {
    value.trim().parse::<IpAddr>().ok()
}

fn is_trusted_proxy(peer_ip: IpAddr, trusted_proxy_cidrs: &[String]) -> bool {
    trusted_proxy_cidrs
        .iter()
        .filter_map(|cidr| cidr.parse::<IpNet>().ok())
        .any(|net| net.contains(&peer_ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn ignores_forwarded_headers_when_peer_is_not_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.10".parse().unwrap());

        let selected = set_trusted_client_ip(
            &mut headers,
            Some("198.51.100.7:443".parse().unwrap()),
            &["10.0.0.0/8".to_string()],
        );

        assert_eq!(selected.as_deref(), Some("198.51.100.7"));
        assert_eq!(client_ip(&headers).as_deref(), Some("198.51.100.7"));
        assert!(!headers.contains_key("x-forwarded-for"));
    }

    #[test]
    fn trusts_forwarded_headers_when_peer_is_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.10, 10.0.0.1".parse().unwrap());

        let selected = set_trusted_client_ip(
            &mut headers,
            Some("10.0.0.5:443".parse().unwrap()),
            &["10.0.0.0/8".to_string()],
        );

        assert_eq!(selected.as_deref(), Some("203.0.113.10"));
        assert_eq!(client_ip(&headers).as_deref(), Some("203.0.113.10"));
    }
}
