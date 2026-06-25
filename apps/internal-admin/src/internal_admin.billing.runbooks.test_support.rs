use axum::{body::Body, http::Request};
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
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

pub(crate) async fn runbook_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
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

pub(crate) async fn seed_workspace_with_actor(pool: &PgPool, actor_id: Uuid) -> (Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Runbook Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("runbook-tenant-{}", Uuid::new_v4()))
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

    insert_workspace(pool, workspace_id, tenant_id).await;
    (tenant_id, workspace_id)
}

async fn insert_workspace(pool: &PgPool, workspace_id: Uuid, tenant_id: Uuid) {
    if workspace_status_column_exists(pool).await {
        sqlx::query(
            "INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code, status)
             VALUES ($1, $2, 'Runbook Workspace', 'team', 'team', 'active')",
        )
        .bind(workspace_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("workspace should insert");
        return;
    }
    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code)
         VALUES ($1, $2, 'Runbook Workspace', 'team', 'team')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("workspace should insert");
}

async fn workspace_status_column_exists(pool: &PgPool) -> bool {
    sqlx::query(
        "SELECT 1 FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = 'workspaces' AND column_name = 'status'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| row.get::<i32, _>(0) == 1)
    .unwrap_or(false)
}

pub(crate) fn execute_runbook_request(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    runbook_id: &str,
) -> Request<Body> {
    execute_runbook_request_with_key(
        workspace_id,
        actor_id,
        role,
        confirm_code,
        runbook_id,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn execute_runbook_request_with_key(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    runbook_id: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/billing/admin/runbooks/{runbook_id}/execute"
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
                "reason": "incident BILL-789 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) async fn runbook_audit_count(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
    runbook_id: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND workspace_id = $2 AND actor_principal_id = $3
           AND action = 'internal_admin.runbook.executed'
           AND target_type = 'runbook'
           AND target_id = $2
           AND metadata->>'runbook_id' = $4",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(runbook_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}
