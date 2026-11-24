//! Detects a significant change of the environment a browser session is used
//! from (OWASP cookie theft mitigation, CTM_REQ_003).
//!
//! A session is bound, when its cookie is issued, to a keyed fingerprint of
//! the client environment: the browser and operating-system families and the
//! network prefix. Versions and the exact address are ignored so routine
//! updates and address churn inside one network keep the session. Raw values
//! are never stored.
//!
//! A later request presenting the cookie from another environment does not
//! revoke anything: the session is flagged `reauth_required_at`, so every
//! freshness decision treats it as stale and the next interactive step asks
//! for a new login, which issues a new, freshly bound session.

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::PgPool;

use crate::{browser::BrowserSecurity, rate_limits::RateLimiter};

const FINGERPRINT_LABEL: &[u8] = b"nvbes.identity.session-context.v1";

#[derive(Clone)]
pub struct SessionContextGuard {
    db: PgPool,
    browser: BrowserSecurity,
    key: [u8; 32],
}

#[derive(Debug, PartialEq, Eq)]
pub enum Observation {
    /// No live session carries this cookie.
    Unknown,
    /// The session had no fingerprint yet and is now bound to this one.
    Bound,
    Unchanged,
    /// The environment changed; the session now requires a new login.
    ReauthenticationRequired,
}

impl SessionContextGuard {
    /// The fingerprint key is derived from the rate-limit key with a distinct
    /// label, so every replica shares it without a separate secret.
    pub fn new(db: PgPool, browser: BrowserSecurity, limiter: &RateLimiter) -> Option<Self> {
        Some(Self {
            db,
            browser,
            key: limiter.derive(FINGERPRINT_LABEL)?,
        })
    }

    pub fn fingerprint(&self, headers: &HeaderMap, peer: Option<IpAddr>) -> Option<[u8; 32]> {
        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).ok()?;
        mac.update(environment_family(user_agent).as_bytes());
        mac.update(&[0]);
        mac.update(network_prefix(peer).as_bytes());
        Some(mac.finalize().into_bytes().into())
    }

    pub async fn observe(
        &self,
        token: &str,
        fingerprint: &[u8; 32],
    ) -> Result<Observation, sqlx::Error> {
        let mut tx = self.db.begin().await?;
        crate::session_locks::session(&mut tx, token).await?;
        let token_hash = crate::auth::hash_token(token);
        let row = sqlx::query!(
            "SELECT id, principal_id, context_fingerprint, reauth_required_at IS NOT NULL AS \"flagged!\" FROM identity_sessions WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > clock_timestamp() FOR UPDATE",
            token_hash,
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(Observation::Unknown);
        };
        let observation = match row.context_fingerprint {
            None => {
                sqlx::query!(
                    "UPDATE identity_sessions SET context_fingerprint = $1 WHERE id = $2",
                    fingerprint.as_slice(),
                    row.id,
                )
                .execute(&mut *tx)
                .await?;
                Observation::Bound
            }
            Some(stored) if stored.as_slice() == fingerprint.as_slice() => Observation::Unchanged,
            Some(_) if row.flagged => Observation::ReauthenticationRequired,
            Some(_) => {
                sqlx::query!(
                    "UPDATE identity_sessions SET reauth_required_at = clock_timestamp() WHERE id = $1 AND reauth_required_at IS NULL",
                    row.id,
                )
                .execute(&mut *tx)
                .await?;
                crate::oauth::store::audit(
                    &mut tx,
                    row.principal_id,
                    "identity.session.context_changed",
                )
                .await?;
                Observation::ReauthenticationRequired
            }
        };
        tx.commit().await?;
        Ok(observation)
    }

    /// Binds a session issued by this very request to the request environment.
    pub async fn bind_issued(
        &self,
        token: &str,
        fingerprint: &[u8; 32],
    ) -> Result<(), sqlx::Error> {
        let token_hash = crate::auth::hash_token(token);
        sqlx::query!(
            "UPDATE identity_sessions SET context_fingerprint = $1 WHERE token_hash = $2 AND context_fingerprint IS NULL",
            fingerprint.as_slice(),
            token_hash,
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

/// Runs on every Identity browser route. A request whose session cannot be
/// checked is refused rather than served with an unverified session.
pub async fn enforce(
    State(guard): State<SessionContextGuard>,
    request: Request,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| peer.ip());
    let Some(fingerprint) = guard.fingerprint(request.headers(), peer) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match guard.browser.session_token(request.headers()) {
        Ok(Some(token)) => {
            if guard.observe(&token, &fingerprint).await.is_err() {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
        }
        Ok(None) => {}
        // Malformed or duplicated cookies are rejected by the route itself.
        Err(_) => {}
    }
    let response = next.run(request).await;
    if let Some(issued) = guard.browser.issued_session_token(response.headers())
        && let Err(error) = guard.bind_issued(&issued, &fingerprint).await
    {
        // The session stays unbound and is bound on its next presentation.
        tracing::warn!(%error, "session context binding deferred");
    }
    response
}

/// Coarse browser and operating-system families; versions are ignored.
pub(crate) fn environment_family(user_agent: &str) -> String {
    let ua = user_agent.to_ascii_lowercase();
    let browser = if ua.contains("edg/") {
        "edge"
    } else if ua.contains("firefox/") || ua.contains("fxios/") {
        "firefox"
    } else if ua.contains("chrome/") || ua.contains("crios/") || ua.contains("chromium/") {
        "chromium"
    } else if ua.contains("safari/") {
        "safari"
    } else {
        "other"
    };
    let os = if ua.contains("android") {
        "android"
    } else if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ios") {
        "ios"
    } else if ua.contains("windows") {
        "windows"
    } else if ua.contains("mac os x") || ua.contains("macintosh") {
        "macos"
    } else if ua.contains("linux") || ua.contains("x11") {
        "linux"
    } else {
        "other"
    };
    format!("{browser}/{os}")
}

/// IPv4 /24 and IPv6 /48 keep the network without the host part.
pub(crate) fn network_prefix(peer: Option<IpAddr>) -> String {
    match peer.map(|ip| ip.to_canonical()) {
        Some(IpAddr::V4(ip)) => {
            let [a, b, c, _] = ip.octets();
            format!("{a}.{b}.{c}.0/24")
        }
        Some(IpAddr::V6(ip)) => {
            let segments = ip.segments();
            format!("{:x}:{:x}:{:x}::/48", segments[0], segments[1], segments[2])
        }
        None => "unknown".into(),
    }
}

#[cfg(test)]
#[path = "identity.session.context.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.session.context.database.tests.rs"]
mod database_tests;
