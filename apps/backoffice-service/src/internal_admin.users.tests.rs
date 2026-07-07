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
    assert!(validate_lifecycle_reason("incident SEC-123 approved").is_ok());
}

#[test]
fn user_lifecycle_rejects_terminal_and_unchanged_statuses() {
    assert!(validate_user_status_transition("deleted", "active", "active", "active").is_err());
    assert!(validate_user_status_transition("revoked", "active", "active", "active").is_err());
    assert!(validate_user_status_transition("active", "deleted", "active", "active").is_err());
    assert!(
        validate_user_status_transition("suspended", "suspended", "suspended", "suspended")
            .is_err()
    );
    assert!(validate_user_status_transition("active", "active", "suspended", "suspended").is_ok());
}

#[tokio::test]
async fn suspend_user_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !user_lifecycle_schema_exists(&pool).await {
        eprintln!("skipping test: user lifecycle schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let tenant_id = seed_tenant_actor_and_user(&pool, actor_id, target_id).await;
    let app = Router::new()
        .merge(router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(suspend_request(
            target_id,
            actor_id,
            "finance_admin",
            &strong_confirmation_code("SUSPEND USER", target_id),
            "ticket SEC-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(suspend_request(
            target_id,
            actor_id,
            "security_admin",
            "SUSPEND",
            "ticket SEC-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(suspend_request(
            target_id,
            actor_id,
            "security_admin",
            &strong_confirmation_code("SUSPEND USER", target_id),
            "ticket SEC-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["next_principal_status"], json!("suspended"));
    assert_eq!(payload["next_user_status"], json!("suspended"));

    let row = sqlx::query(
        "SELECT p.status::text AS principal_status, u.status::text AS user_status
         FROM principals p JOIN users u ON u.principal_id = p.id
         WHERE p.id = $1",
    )
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("target user should exist");
    let principal_status: String = row.get("principal_status");
    let user_status: String = row.get("user_status");
    assert_eq!(principal_status, "suspended");
    assert_eq!(user_status, "suspended");

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'internal_admin.user.suspend'
           AND target_type = 'principal'
           AND target_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(target_id)
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

async fn user_lifecycle_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.users') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_tenant_actor_and_user(pool: &PgPool, actor_id: Uuid, target_id: Uuid) -> Uuid {
    let tenant_id = Uuid::new_v4();
    let slug = format!("test-user-tenant-{}", Uuid::new_v4());
    let email = format!("{}@example.test", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Test User Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(slug)
    .execute(pool)
    .await
    .expect("tenant should insert");

    for (principal_id, display_name) in [(actor_id, "Backoffice Actor"), (target_id, "Target User")]
    {
        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
             VALUES ($1, $2, 'human', 'active', $3)",
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(display_name)
        .execute(pool)
        .await
        .expect("principal should insert");
    }

    sqlx::query(
        "INSERT INTO users (principal_id, email, name, status, email_verified_at)
         VALUES ($1, $2, 'Target User', 'active', NOW())",
    )
    .bind(target_id)
    .bind(email)
    .execute(pool)
    .await
    .expect("user should insert");

    tenant_id
}

fn suspend_request(
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/admin/users/{principal_id}/suspend"))
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
