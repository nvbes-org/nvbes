use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request},
    middleware::Next,
    response::Response,
};
use nvbes_core::limiter::RateLimitInfo;

use crate::app::AppState;
use crate::error::AppError;

const ACTOR_HEADER: &str = "x-nvbes-actor-principal-id";
const CLIENT_IP_HEADER: &str = "x-nvbes-client-ip";

const READ_ACTOR_LIMIT: usize = 240;
const READ_IP_LIMIT: usize = 600;
const MUTATION_ACTOR_LIMIT: usize = 20;
const MUTATION_IP_LIMIT: usize = 60;
const CRITICAL_MUTATION_ACTOR_LIMIT: usize = 5;
const CRITICAL_MUTATION_IP_LIMIT: usize = 20;
const WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
pub struct BackofficeRateLimiter {
    buckets: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl BackofficeRateLimiter {
    pub fn check(
        &self,
        key: &str,
        limit: usize,
        window: Duration,
    ) -> Result<RateLimitInfo, AppError> {
        let now = Instant::now();
        let mut buckets = self
            .buckets
            .lock()
            .expect("rate limiter mutex should not poison");
        let bucket = buckets.entry(key.to_string()).or_default();
        while bucket
            .front()
            .is_some_and(|hit| now.duration_since(*hit) >= window)
        {
            bucket.pop_front();
        }
        if bucket.len() >= limit {
            let reset = bucket
                .front()
                .map(|first| {
                    window
                        .saturating_sub(now.duration_since(*first))
                        .as_secs()
                        .max(1)
                })
                .unwrap_or(1);
            return Err(AppError::too_many_requests(
                "backoffice_rate_limited",
                "Too many back-office requests. Try again later.",
                Some(reset),
                Some(RateLimitInfo {
                    limit,
                    remaining: 0,
                    reset,
                }),
            ));
        }
        bucket.push_back(now);
        Ok(RateLimitInfo {
            limit,
            remaining: limit.saturating_sub(bucket.len()),
            reset: window.as_secs(),
        })
    }
}

pub async fn backoffice_rate_limit(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let policy = policy_for_request(request.method(), request.uri().path());
    let actor_key = actor_key(&headers);
    let ip_key = ip_key(&headers);

    state.rate_limiter.check(
        &format!("actor:{actor_key}:{}", policy.kind),
        policy.actor_limit,
        WINDOW,
    )?;
    state.rate_limiter.check(
        &format!("ip:{ip_key}:{}", policy.kind),
        policy.ip_limit,
        WINDOW,
    )?;

    Ok(next.run(request).await)
}

struct BackofficeRatePolicy {
    actor_limit: usize,
    ip_limit: usize,
    kind: &'static str,
}

fn policy_for_request(method: &Method, path: &str) -> BackofficeRatePolicy {
    if matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    ) {
        if is_critical_mutation_path(path) {
            return BackofficeRatePolicy {
                actor_limit: CRITICAL_MUTATION_ACTOR_LIMIT,
                ip_limit: CRITICAL_MUTATION_IP_LIMIT,
                kind: "critical_mutation",
            };
        }
        return BackofficeRatePolicy {
            actor_limit: MUTATION_ACTOR_LIMIT,
            ip_limit: MUTATION_IP_LIMIT,
            kind: "mutation",
        };
    }
    BackofficeRatePolicy {
        actor_limit: READ_ACTOR_LIMIT,
        ip_limit: READ_IP_LIMIT,
        kind: "read",
    }
}

fn is_critical_mutation_path(path: &str) -> bool {
    let is_region_exception = path.contains("/region/workspaces/") && path.ends_with("/exceptions");
    let is_invoice_hold = path.contains("/invoices/") && path.ends_with("/hold");
    let is_routing_disable = path.contains("/routing-rules/") && path.ends_with("/disable");
    let is_kyc_reject = path.contains("/kyc-profiles/") && path.ends_with("/reject");
    let is_risk_block = path.contains("/risk/policies/") && path.ends_with("/block");
    let is_security_revoke = path.contains("/admin/security-center/")
        && (path.ends_with("/revoke") || path.ends_with("/cancel"));
    let is_governance_revoke = path.contains("/admin/identity-governance-center/")
        && (path.ends_with("/revoke") || path.ends_with("/cancel"));
    let is_access_suspend = path.contains("/admin/access-center/") && path.ends_with("/suspend");

    path.ends_with("/erasure-request")
        || is_region_exception
        || is_invoice_hold
        || is_routing_disable
        || is_kyc_reject
        || is_risk_block
        || is_security_revoke
        || is_governance_revoke
        || is_access_suspend
}

fn actor_key(headers: &HeaderMap) -> String {
    headers
        .get(ACTOR_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("anonymous")
        .to_string()
}

fn ip_key(headers: &HeaderMap) -> String {
    headers
        .get(CLIENT_IP_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    #[test]
    fn mutation_policy_is_stricter_than_read_policy() {
        let read = policy_for_request(&Method::GET, "/admin/command-center");
        let mutation = policy_for_request(&Method::POST, "/admin/operations/replay");

        assert!(mutation.actor_limit < read.actor_limit);
        assert!(mutation.ip_limit < read.ip_limit);
    }

    #[test]
    fn critical_mutation_policy_is_stricter_than_regular_mutation_policy() {
        let mutation = policy_for_request(&Method::POST, "/admin/operations/replay");
        let critical = policy_for_request(
            &Method::POST,
            "/workspaces/00000000-0000-0000-0000-000000000001/admin/revenue/invoices/00000000-0000-0000-0000-000000000002/hold",
        );
        let security_critical = policy_for_request(
            &Method::POST,
            "/admin/security-center/mfa-factors/00000000-0000-0000-0000-000000000001/revoke",
        );
        let governance_critical = policy_for_request(
            &Method::POST,
            "/admin/identity-governance-center/recovery-requests/00000000-0000-0000-0000-000000000001/cancel",
        );
        let access_critical = policy_for_request(
            &Method::POST,
            "/admin/access-center/workspace-memberships/00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002/suspend",
        );

        assert!(critical.actor_limit < mutation.actor_limit);
        assert!(critical.ip_limit < mutation.ip_limit);
        assert_eq!(critical.kind, "critical_mutation");
        assert_eq!(security_critical.kind, "critical_mutation");
        assert_eq!(governance_critical.kind, "critical_mutation");
        assert_eq!(access_critical.kind, "critical_mutation");
    }

    #[test]
    fn limiter_blocks_after_limit() {
        let limiter = BackofficeRateLimiter::default();
        assert!(limiter.check("actor:a:mutation", 1, WINDOW).is_ok());
        let error = limiter
            .check("actor:a:mutation", 1, WINDOW)
            .expect_err("second hit should be rate limited");

        assert_eq!(error.code, "backoffice_rate_limited");
    }

    #[tokio::test]
    async fn post_requests_are_rate_limited_through_http_middleware() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/internal_admin_rate_limit_test")
            .expect("lazy pool should build");
        let state = AppState::new(nvbes_core::config::AppConfig::default(), pool);
        let app = Router::new()
            .route("/probe", post(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                backoffice_rate_limit,
            ));

        for _ in 0..MUTATION_ACTOR_LIMIT {
            let response = app
                .clone()
                .oneshot(post_request())
                .await
                .expect("middleware should respond");
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app
            .oneshot(post_request())
            .await
            .expect("middleware should rate limit");

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("retry-after"));
        let limit = MUTATION_ACTOR_LIMIT.to_string();
        assert_eq!(
            response
                .headers()
                .get("ratelimit-limit")
                .and_then(|value| value.to_str().ok()),
            Some(limit.as_str())
        );
    }

    #[tokio::test]
    async fn critical_mutations_are_rate_limited_with_dedicated_policy() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/internal_admin_rate_limit_test")
            .expect("lazy pool should build");
        let state = AppState::new(nvbes_core::config::AppConfig::default(), pool);
        let path = "/workspaces/00000000-0000-0000-0000-000000000001/admin/revenue/invoices/00000000-0000-0000-0000-000000000002/hold";
        let app = Router::new().route(path, post(|| async { "ok" })).layer(
            axum::middleware::from_fn_with_state(state.clone(), backoffice_rate_limit),
        );

        for _ in 0..CRITICAL_MUTATION_ACTOR_LIMIT {
            let response = app
                .clone()
                .oneshot(post_request_to(path))
                .await
                .expect("middleware should respond");
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app
            .oneshot(post_request_to(path))
            .await
            .expect("middleware should rate limit critical mutation");

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let limit = CRITICAL_MUTATION_ACTOR_LIMIT.to_string();
        assert_eq!(
            response
                .headers()
                .get("ratelimit-limit")
                .and_then(|value| value.to_str().ok()),
            Some(limit.as_str())
        );
    }

    fn post_request() -> Request<Body> {
        post_request_to("/probe")
    }

    fn post_request_to(path: &str) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(ACTOR_HEADER, "00000000-0000-0000-0000-000000000001")
            .header(CLIENT_IP_HEADER, "127.0.0.1")
            .body(Body::empty())
            .expect("request should build")
    }
}
