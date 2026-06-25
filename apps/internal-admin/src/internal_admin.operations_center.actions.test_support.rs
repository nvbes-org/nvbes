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

pub(crate) async fn operations_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.billing_provider_events') IS NOT NULL
          AND to_regclass('public.internal_admin_operations_actions') IS NOT NULL
          AND to_regclass('public.internal_admin_incidents') IS NOT NULL
          AND to_regclass('public.internal_admin_maintenance_windows') IS NOT NULL
          AND to_regclass('public.internal_admin_job_runs') IS NOT NULL",
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

pub(crate) async fn seed_provider_event_with_actor(
    pool: &PgPool,
    actor_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Operations Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("operations-tenant-{}", Uuid::new_v4()))
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

    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, name, slug, workspace_type, plan_code)
         VALUES ($1, $2, 'Operations Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("operations-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    sqlx::query(
        "INSERT INTO billing_provider_events (
           id, tenant_id, provider, provider_event_id, event_type, status, payload_hash
         ) VALUES ($1, $2, 'stripe', $3, 'invoice.payment_failed', 'failed', $4)",
    )
    .bind(event_id)
    .bind(tenant_id)
    .bind(format!("evt_{}", Uuid::new_v4()))
    .bind(Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .expect("provider event should insert");

    (tenant_id, workspace_id, event_id)
}

pub(crate) fn replay_request(
    workspace_id: Uuid,
    event_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    replay_request_with_key(
        workspace_id,
        event_id,
        actor_id,
        role,
        confirm_code,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn replay_request_with_key(
    workspace_id: Uuid,
    event_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/operations/provider-events/{event_id}/replay"
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
                "reason": "ticket OPS-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}
