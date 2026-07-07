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
use crate::observability::{record_action_request, record_rate_limited};

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
    let method = request.method().as_str().to_string();
    let family = action_family_for_path(request.uri().path());
    let actor_key = actor_key(&headers);
    let ip_key = ip_key(&headers);

    let actor_info = state
        .rate_limiter
        .check(
            &format!("actor:{actor_key}:{}", policy.kind),
            policy.actor_limit,
            WINDOW,
        )
        .map_err(|error| {
            record_rate_limited(policy.kind, "actor");
            error
        })?;
    let ip_info = state
        .rate_limiter
        .check(
            &format!("ip:{ip_key}:{}", policy.kind),
            policy.ip_limit,
            WINDOW,
        )
        .map_err(|error| {
            record_rate_limited(policy.kind, "ip");
            error
        })?;

    let started_at = Instant::now();
    let mut response = next.run(request).await;
    record_action_request(
        family,
        &method,
        policy.kind,
        response.status().as_u16(),
        started_at.elapsed().as_secs_f64(),
    );
    effective_rate_limit_info(&actor_info, &ip_info).append_headers(response.headers_mut());
    Ok(response)
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
    if is_backoffice_action_path(path) {
        return !path.contains("/billing/admin/exports/");
    }

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

fn is_backoffice_action_path(path: &str) -> bool {
    path.starts_with("/admin/") || path.contains("/admin/")
}

fn action_family_for_path(path: &str) -> &'static str {
    if path.contains("/billing/admin/") {
        return "billing_admin";
    }
    if path.contains("/admin/access-center/") {
        return "access_center";
    }
    if path.contains("/admin/audit") {
        return "audit";
    }
    if path.contains("/admin/billing-platform-center/") {
        return "billing_platform_center";
    }
    if path.contains("/admin/communications-center/") {
        return "communications_center";
    }
    if path.contains("/admin/compliance/") {
        return "compliance_center";
    }
    if path.contains("/admin/developer-center/") {
        return "developer_center";
    }
    if path.contains("/admin/entitlements-center/") {
        return "entitlements_center";
    }
    if path.contains("/admin/identity-governance-center/") {
        return "identity_governance_center";
    }
    if path.contains("/admin/operations-center/") {
        return "operations_center";
    }
    if path.contains("/admin/region/") {
        return "region_center";
    }
    if path.contains("/admin/revenue/") {
        return "revenue_center";
    }
    if path.contains("/admin/risk/") {
        return "risk_decision_center";
    }
    if path.contains("/admin/security-center/") {
        return "security_center";
    }
    if path.contains("/admin/usage-center/") {
        return "usage_center";
    }
    if path.contains("/admin/users/") {
        return "user_lifecycle";
    }
    if path.contains("/admin/workspaces/") {
        return "workspace_lifecycle";
    }
    if path.contains("/admin/tenants/") {
        return "tenant_lifecycle";
    }
    "platform"
}

fn effective_rate_limit_info<'a>(
    actor_info: &'a RateLimitInfo,
    ip_info: &'a RateLimitInfo,
) -> &'a RateLimitInfo {
    if ip_info.remaining < actor_info.remaining {
        return ip_info;
    }
    actor_info
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
#[path = "internal_admin.rate_limit.tests.rs"]
mod tests;
