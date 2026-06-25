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
    let mutation = policy_for_request(&Method::POST, "/probe");

    assert!(mutation.actor_limit < read.actor_limit);
    assert!(mutation.ip_limit < read.ip_limit);
}

#[test]
fn critical_mutation_policy_is_stricter_than_regular_mutation_policy() {
    let mutation = policy_for_request(&Method::POST, "/probe");
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
    let compliance_critical = policy_for_request(
        &Method::POST,
        "/workspaces/00000000-0000-0000-0000-000000000001/admin/compliance/consents/00000000-0000-0000-0000-000000000002/revoke",
    );
    let billing_admin_critical = policy_for_request(
        &Method::POST,
        "/workspaces/00000000-0000-0000-0000-000000000001/billing/admin/credit-notes",
    );
    let export = policy_for_request(
        &Method::POST,
        "/workspaces/00000000-0000-0000-0000-000000000001/billing/admin/exports/invoices",
    );

    assert!(critical.actor_limit < mutation.actor_limit);
    assert!(critical.ip_limit < mutation.ip_limit);
    assert_eq!(critical.kind, "critical_mutation");
    assert_eq!(security_critical.kind, "critical_mutation");
    assert_eq!(governance_critical.kind, "critical_mutation");
    assert_eq!(access_critical.kind, "critical_mutation");
    assert_eq!(compliance_critical.kind, "critical_mutation");
    assert_eq!(billing_admin_critical.kind, "critical_mutation");
    assert_eq!(export.kind, "mutation");
}

#[test]
fn action_family_uses_low_cardinality_route_groups() {
    assert_eq!(
        action_family_for_path("/workspaces/tenant-1/billing/admin/credit-notes"),
        "billing_admin"
    );
    assert_eq!(
        action_family_for_path("/admin/revenue/invoices/00000000-0000-0000-0000-000000000001/hold"),
        "revenue_center"
    );
    assert_eq!(action_family_for_path("/admin/command-center"), "platform");
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
    let app = test_app("/probe");

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
async fn successful_responses_include_rate_limit_headers() {
    let response = test_app("/probe")
        .oneshot(post_request())
        .await
        .expect("middleware should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let limit = MUTATION_ACTOR_LIMIT.to_string();
    assert_eq!(
        response
            .headers()
            .get("ratelimit-limit")
            .and_then(|value| value.to_str().ok()),
        Some(limit.as_str())
    );
    assert!(response.headers().contains_key("ratelimit-remaining"));
    assert!(response.headers().contains_key("ratelimit-reset"));
}

#[tokio::test]
async fn rate_limit_is_partitioned_by_actor_before_ip_limit() {
    let app = test_app("/probe");

    for _ in 0..MUTATION_ACTOR_LIMIT {
        let response = app
            .clone()
            .oneshot(post_request_with_actor(
                "00000000-0000-0000-0000-000000000001",
                "127.0.0.1",
            ))
            .await
            .expect("middleware should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let same_actor = app
        .clone()
        .oneshot(post_request_with_actor(
            "00000000-0000-0000-0000-000000000001",
            "127.0.0.1",
        ))
        .await
        .expect("middleware should rate limit the actor");
    assert_eq!(same_actor.status(), StatusCode::TOO_MANY_REQUESTS);

    let different_actor = app
        .oneshot(post_request_with_actor(
            "00000000-0000-0000-0000-000000000002",
            "127.0.0.1",
        ))
        .await
        .expect("different actor should still be allowed before IP limit");
    assert_eq!(different_actor.status(), StatusCode::OK);
}

#[tokio::test]
async fn critical_mutations_are_rate_limited_with_dedicated_policy() {
    let path = "/workspaces/00000000-0000-0000-0000-000000000001/admin/revenue/invoices/00000000-0000-0000-0000-000000000002/hold";
    let app = test_app(path);

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

fn test_app(path: &'static str) -> Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_rate_limit_test")
        .expect("lazy pool should build");
    let state = AppState::new(nvbes_core::config::AppConfig::default(), pool);
    Router::new()
        .route(path, post(|| async { "ok" }))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            backoffice_rate_limit,
        ))
}

fn post_request() -> Request<Body> {
    post_request_to("/probe")
}

fn post_request_to(path: &str) -> Request<Body> {
    post_request_to_with_actor(path, "00000000-0000-0000-0000-000000000001", "127.0.0.1")
}

fn post_request_with_actor(actor_id: &str, ip: &str) -> Request<Body> {
    post_request_to_with_actor("/probe", actor_id, ip)
}

fn post_request_to_with_actor(path: &str, actor_id: &str, ip: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(ACTOR_HEADER, actor_id)
        .header(CLIENT_IP_HEADER, ip)
        .body(Body::empty())
        .expect("request should build")
}
