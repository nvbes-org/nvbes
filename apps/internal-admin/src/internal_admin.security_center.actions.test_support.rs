use axum::{body::Body, http::Request};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use uuid::Uuid;

pub(crate) async fn test_pool() -> Option<PgPool> {
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

pub(crate) async fn mfa_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.users') IS NOT NULL
          AND to_regclass('public.mfa_factors') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub(crate) async fn idempotency_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.idempotency_responses') IS NOT NULL")
        .fetch_one(pool)
        .await
        .unwrap_or(false)
}

pub(crate) async fn seed_tenant_actor_user_and_mfa(
    pool: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
    factor_id: Uuid,
) -> Uuid {
    let tenant_id = Uuid::new_v4();
    let slug = format!("test-security-tenant-{}", Uuid::new_v4());
    let email = format!("{}@example.test", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Test Security Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(slug)
    .execute(pool)
    .await
    .expect("tenant should insert");

    for (id, display_name) in [
        (actor_id, "Backoffice Actor"),
        (principal_id, "Target User"),
    ] {
        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
             VALUES ($1, $2, 'human', 'active', $3)",
        )
        .bind(id)
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
    .bind(principal_id)
    .bind(email)
    .execute(pool)
    .await
    .expect("user should insert");

    sqlx::query(
        "INSERT INTO mfa_factors (
           id, principal_id, factor_type, status, label, confirmed_at
         ) VALUES ($1, $2, 'totp', 'active', 'Test TOTP', NOW())",
    )
    .bind(factor_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("MFA factor should insert");

    tenant_id
}

pub(crate) fn revoke_mfa_request(
    factor_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    revoke_mfa_request_with_key(
        factor_id,
        actor_id,
        role,
        confirm_code,
        reason,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn revoke_mfa_request_with_key(
    factor_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/admin/security-center/mfa-factors/{factor_id}/revoke"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", idempotency_key)
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

pub(crate) fn revoke_mfa_request_without_second_approver(
    factor_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/admin/security-center/mfa-factors/{factor_id}/revoke"
        ))
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
