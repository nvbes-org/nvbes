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

pub(crate) async fn governance_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.tenant_break_glass_accounts') IS NOT NULL
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

pub(crate) async fn seed_break_glass_account(
    pool: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
) -> Uuid {
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Governance Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("governance-tenant-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("tenant should insert");

    for (id, display_name) in [
        (actor_id, "Backoffice Actor"),
        (principal_id, "Break Glass User"),
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
        "INSERT INTO tenant_break_glass_accounts (
           tenant_id, principal_id, procedure_reference, reason, created_by_principal_id
         ) VALUES ($1, $2, 'BG-123', 'incident response account', $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("break-glass account should insert");

    tenant_id
}

pub(crate) fn revoke_break_glass_request(
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    revoke_break_glass_request_with_key(
        tenant_id,
        principal_id,
        actor_id,
        role,
        confirm_code,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn revoke_break_glass_request_with_key(
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/admin/identity-governance-center/break-glass/{tenant_id}/{principal_id}/revoke"
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
                "reason": "ticket GOV-456 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) fn revoke_break_glass_request_without_second_approver(
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/admin/identity-governance-center/break-glass/{tenant_id}/{principal_id}/revoke"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": "ticket GOV-456 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) async fn break_glass_revoked(
    pool: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM tenant_break_glass_accounts
         WHERE tenant_id = $1 AND principal_id = $2",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("break-glass account should load")
}

pub(crate) async fn break_glass_audit_count(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    principal_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'internal_admin.identity_governance.break_glass.revoked'
           AND target_type = 'break_glass_account'
           AND target_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}
