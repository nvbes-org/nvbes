use reqwest::Url;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::lookup_host;

use crate::http::error::AppError;

pub(crate) async fn fetch_federation_url(
    value: &str,
    strict_mode: bool,
) -> Result<reqwest::Response, AppError> {
    let url = Url::parse(value.trim()).map_err(|_| {
        AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL is invalid.",
        )
    })?;

    if url.username() != "" || url.password().is_some() {
        return Err(AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL cannot contain credentials.",
        ));
    }

    if url.host_str().is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL must include a host.",
        ));
    }

    if !matches!(url.scheme(), "https") && strict_mode {
        return Err(AppError::bad_request(
            "validation_failed",
            "Federation endpoints must use HTTPS in non-development environments.",
        ));
    }

    if !strict_mode {
        return request(value).await;
    }

    let host = url.host_str().unwrap();

    if let Ok(ip_addr) = host.parse::<IpAddr>() {
        if !is_public_ip(ip_addr) {
            return Err(AppError::bad_request(
                "validation_failed",
                "Federation endpoints cannot target private or loopback addresses.",
            ));
        }
        return request(value).await;
    }

    let ip = resolve_host(host, url.port_or_known_default().unwrap_or(443)).await?;
    request_with_pinned_ip(value, host, ip).await
}

pub(crate) fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            !ipv4.is_private()
                && !ipv4.is_loopback()
                && !ipv4.is_link_local()
                && !ipv4.is_multicast()
                && !ipv4.is_broadcast()
                && !ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            !ipv6.is_loopback()
                && !ipv6.is_unique_local()
                && !ipv6.is_unicast_link_local()
                && !ipv6.is_multicast()
                && !ipv6.is_unspecified()
        }
    }
}

async fn resolve_host(host: &str, port: u16) -> Result<IpAddr, AppError> {
    let mut resolved = lookup_host((host, port)).await.map_err(|_| {
        AppError::bad_request(
            "validation_failed",
            "The federation endpoint host could not be resolved.",
        )
    })?;

    let Some(addr) = resolved.find(|addr| is_public_ip(addr.ip())) else {
        return Err(AppError::bad_request(
            "validation_failed",
            "Federation endpoints cannot resolve to private or loopback addresses.",
        ));
    };

    Ok(addr.ip())
}

async fn request_with_pinned_ip(
    url: &str,
    host: &str,
    ip: IpAddr,
) -> Result<reqwest::Response, AppError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .resolve(host, SocketAddr::new(ip, 0))
        .build()
        .map_err(|err| AppError::internal("federation_client_unavailable", &format!("{}", err)))?;

    client
        .get(url)
        .send()
        .await
        .map_err(|err| AppError::internal("federation_request_failed", &format!("{}", err)))
}

async fn request(url: &str) -> Result<reqwest::Response, AppError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| AppError::internal("federation_client_unavailable", &format!("{}", err)))?;

    client
        .get(url)
        .send()
        .await
        .map_err(|err| AppError::internal("federation_request_failed", &format!("{}", err)))
}
