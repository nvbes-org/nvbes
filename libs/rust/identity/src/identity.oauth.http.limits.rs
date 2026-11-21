use super::{OAuthError, ProtocolError};
use crate::{
    browser::BrowserSecurity,
    rate_limits::{Category, RateLimiter},
};
use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

#[derive(Clone)]
pub(super) struct SourceLimit {
    pub db: PgPool,
    pub limiter: RateLimiter,
    pub browser: Option<BrowserSecurity>,
}

use crate::oauth::limits::enforce;

pub(super) async fn protect_source(
    State(state): State<SourceLimit>,
    request: Request,
    next: Next,
) -> Response {
    // Only the transport can supply this extension. Forwarded/X-Forwarded-For
    // and CDN headers remain untrusted, including on loopback connections.
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| peer.0);
    let mut result = match peer {
        Some(peer) => {
            enforce(
                &state.db,
                &state.limiter,
                Category::ProtocolSource,
                &source(peer.ip()),
            )
            .await
        }
        None => Err(OAuthError::Unavailable),
    };
    if result.is_ok()
        && (matches!(
            request.uri().path(),
            "/oauth/authorize/login" | "/oauth/session/step-up/totp"
        ) || request.uri().path().starts_with("/oauth/session/webauthn/")
            || request.uri().path().starts_with("/oauth/session/totp/")
            || request.uri().path().starts_with("/oauth/session/recovery/")
            || request.uri().path().starts_with("/oauth/recovery/")
            || request
                .uri()
                .path()
                .starts_with("/oauth/authorize/passkey/"))
    {
        result = enforce(
            &state.db,
            &state.limiter,
            Category::LoginSource,
            &source(peer.unwrap().ip()),
        )
        .await;
    }
    let mut response = match result {
        Ok(()) => next.run(request).await,
        Err(error) => ProtocolError::OAuth(error).into_response(),
    };
    if let Some(browser) = state.browser {
        response.headers_mut().extend(browser.response_headers());
    }
    response
}

fn source(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(ip) => ip.to_string(),
            None => {
                let [a, b, c, d, _, _, _, _] = ip.segments();
                format!("{}/64", Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peers_ignore_ports_and_group_ipv6_privacy_addresses() {
        assert_eq!(
            source("127.0.0.1".parse().unwrap()),
            source("::ffff:127.0.0.1".parse().unwrap())
        );
        assert_eq!(
            source("2001:db8:1:2::1".parse().unwrap()),
            source("2001:db8:1:2::ffff".parse().unwrap())
        );
        assert_ne!(
            source("2001:db8:1:2::1".parse().unwrap()),
            source("2001:db8:1:3::1".parse().unwrap())
        );
    }
}
