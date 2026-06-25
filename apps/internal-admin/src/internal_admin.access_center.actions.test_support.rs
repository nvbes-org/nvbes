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

pub(crate) async fn access_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.workspace_memberships') IS NOT NULL
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

pub(crate) async fn seed_workspace_membership(
    pool: &PgPool,
    actor_id: Uuid,
    target_id: Uuid,
    owner_id: Uuid,
) -> (Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Access Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("access-tenant-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("tenant should insert");

    for (id, display_name) in [
        (actor_id, "Backoffice Actor"),
        (target_id, "Target User"),
        (owner_id, "Workspace Owner"),
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
        "INSERT INTO workspaces (id, tenant_id, name, slug, workspace_type, plan_code)
         VALUES ($1, $2, 'Access Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("access-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    for (principal_id, role) in [(owner_id, "owner"), (target_id, "admin")] {
        sqlx::query(
            "INSERT INTO workspace_memberships (workspace_id, principal_id, role, status)
             VALUES ($1, $2, $3::workspace_member_role, 'active')",
        )
        .bind(workspace_id)
        .bind(principal_id)
        .bind(role)
        .execute(pool)
        .await
        .expect("membership should insert");
    }

    (tenant_id, workspace_id)
}

pub(crate) fn suspend_request(
    workspace_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    suspend_request_with_key(
        workspace_id,
        principal_id,
        actor_id,
        role,
        confirm_code,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn suspend_request_with_key(
    workspace_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/admin/access-center/workspace-memberships/{workspace_id}/{principal_id}/suspend"
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
                "reason": "ticket IAM-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}
