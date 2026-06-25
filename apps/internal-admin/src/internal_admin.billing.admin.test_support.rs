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

pub(crate) async fn billing_replay_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.billing_provider_events') IS NOT NULL
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

pub(crate) async fn seed_workspace_actor_and_provider_event(
    pool: &PgPool,
    actor_id: Uuid,
    provider_event_id: &str,
) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Test Billing Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("test-billing-tenant-{}", Uuid::new_v4()))
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
    sqlx::query(
        "INSERT INTO billing_provider_events (
           id, tenant_id, provider, provider_event_id, event_type, status,
           signature_valid, payload_hash, payload_summary
         ) VALUES (
           $1, $2, 'stripe', $3, 'invoice.payment_failed', 'failed',
           true, 'hash-test', '{}'::jsonb
         )",
    )
    .bind(event_id)
    .bind(tenant_id)
    .bind(provider_event_id)
    .execute(pool)
    .await
    .expect("provider event should insert");

    (tenant_id, workspace_id, event_id)
}

async fn insert_workspace(pool: &PgPool, workspace_id: Uuid, tenant_id: Uuid) {
    if workspace_status_column_exists(pool).await {
        sqlx::query(
            "INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code, status)
             VALUES ($1, $2, 'Billing Workspace', 'team', 'team', 'active')",
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
         VALUES ($1, $2, 'Billing Workspace', 'team', 'team')",
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

pub(crate) fn replay_request(
    workspace_id: Uuid,
    actor_id: Uuid,
    approver_id: Option<Uuid>,
    role: &str,
    confirm_code: &str,
    provider_event_id: &str,
    reason: &str,
) -> Request<Body> {
    replay_request_with_key(
        workspace_id,
        actor_id,
        approver_id,
        role,
        confirm_code,
        provider_event_id,
        reason,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn replay_request_with_key(
    workspace_id: Uuid,
    actor_id: Uuid,
    approver_id: Option<Uuid>,
    role: &str,
    confirm_code: &str,
    provider_event_id: &str,
    reason: &str,
    idempotency_key: &str,
) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/billing/admin/provider-events/replay"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", idempotency_key)
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role);
    if let Some(approver_id) = approver_id {
        request = request
            .header(
                "x-nvbes-second-approver-principal-id",
                approver_id.to_string(),
            )
            .header("x-nvbes-second-approver-role", "platform_admin");
    }
    request
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "provider": "stripe",
                "provider_event_id": provider_event_id,
                "reason": reason
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) async fn provider_event_status(pool: &PgPool, event_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM billing_provider_events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("provider event should exist")
}

pub(crate) async fn replay_audit_count(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    event_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'billing.provider_event.replayed'
           AND target_type = 'billing_provider_event'
           AND target_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}

pub(crate) async fn replay_audit_metadata(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    event_id: Uuid,
) -> serde_json::Value {
    sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'billing.provider_event.replayed'
           AND target_type = 'billing_provider_event'
           AND target_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("audit metadata should load")
}
