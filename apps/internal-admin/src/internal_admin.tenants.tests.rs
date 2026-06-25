use super::*;
use crate::backoffice_authorization::strong_confirmation_code;
use axum::{
    Router,
    body::{self, Body},
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower::ServiceExt;

#[test]
fn lifecycle_reason_must_be_detailed() {
    assert!(validate_lifecycle_reason("too short").is_err());
    assert!(validate_lifecycle_reason("ticket SEC-123 approved").is_ok());
}

#[test]
fn tenant_lifecycle_rejects_deleted_and_unchanged_statuses() {
    assert!(validate_tenant_status_transition("deleted", "active").is_err());
    assert!(validate_tenant_status_transition("active", "active").is_err());
    assert!(validate_tenant_status_transition("active", "suspended").is_ok());
}

#[tokio::test]
async fn suspend_tenant_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !tenant_lifecycle_schema_exists(&pool).await {
        eprintln!("skipping test: tenant lifecycle schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let tenant_id = seed_tenant_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(suspend_request(
            tenant_id,
            actor_id,
            "viewer",
            &strong_confirmation_code("SUSPEND TENANT", tenant_id),
            "ticket SEC-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(suspend_request(
            tenant_id,
            actor_id,
            "platform_admin",
            "SUSPEND",
            "ticket SEC-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_dual_control = app
        .clone()
        .oneshot(suspend_request_without_second_approver(
            tenant_id,
            actor_id,
            "platform_admin",
            &strong_confirmation_code("SUSPEND TENANT", tenant_id),
            "ticket SEC-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(suspend_request(
            tenant_id,
            actor_id,
            "platform_admin",
            &strong_confirmation_code("SUSPEND TENANT", tenant_id),
            "ticket SEC-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["next_status"], json!("suspended"));

    let status = sqlx::query_scalar::<_, String>("SELECT status::text FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&pool)
        .await
        .expect("tenant should exist");
    assert_eq!(status, "suspended");

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'internal_admin.tenant.suspend'
           AND target_type = 'tenant'
           AND target_id = $1",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);
}

async fn test_pool() -> Option<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
    tokio::time::timeout(
        Duration::from_secs(2),
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url),
    )
    .await
    .ok()
    .and_then(Result::ok)
}

async fn tenant_lifecycle_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_tenant_with_actor(pool: &PgPool, actor_id: Uuid) -> Uuid {
    let tenant_id = Uuid::new_v4();
    let slug = format!("test-tenant-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Test Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(slug)
    .execute(pool)
    .await
    .expect("tenant should insert");

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
         VALUES ($1, $2, 'human', 'active', 'Backoffice Actor')",
    )
    .bind(actor_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("actor should insert");

    tenant_id
}

fn suspend_request(
    tenant_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/admin/tenants/{tenant_id}/suspend"))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .header(
            "x-nvbes-second-approver-principal-id",
            Uuid::new_v4().to_string(),
        )
        .header("x-nvbes-second-approver-role", "platform_admin")
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": reason
            })
            .to_string(),
        ))
        .expect("request should build")
}

fn suspend_request_without_second_approver(
    tenant_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/admin/tenants/{tenant_id}/suspend"))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": reason
            })
            .to_string(),
        ))
        .expect("request should build")
}
