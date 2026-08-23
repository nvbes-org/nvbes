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

pub(crate) async fn risk_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.billing_risk_signals') IS NOT NULL
          AND to_regclass('public.internal_admin_risk_actions') IS NOT NULL",
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

pub(crate) async fn seed_risk_signal_with_actor(
    pool: &PgPool,
    actor_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let signal_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Risk Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("risk-tenant-{}", Uuid::new_v4()))
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
         VALUES ($1, $2, 'Risk Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("risk-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    sqlx::query(
        "INSERT INTO billing_risk_signals (id, tenant_id, signal_type, signal_value)
         VALUES ($1, $2, 'velocity_spike', '{}'::jsonb)",
    )
    .bind(signal_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("risk signal should insert");

    (tenant_id, workspace_id, signal_id)
}

pub(crate) fn block_policy_request(
    workspace_id: Uuid,
    policy_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/risk/policies/{policy_id}/block"
        ))
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
                "reason": "ticket RISK-123 reviewed"
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) fn resolve_request(
    workspace_id: Uuid,
    signal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    resolve_request_with_key(
        workspace_id,
        signal_id,
        actor_id,
        role,
        confirm_code,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn resolve_request_with_key(
    workspace_id: Uuid,
    signal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/risk/signals/{signal_id}/resolve"
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
                "reason": "ticket RISK-123 reviewed"
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) fn resolve_request_without_second_approver(
    workspace_id: Uuid,
    signal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/risk/signals/{signal_id}/resolve"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": "ticket RISK-123 reviewed"
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) async fn risk_action_count(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    signal_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_risk_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'resolve_risk_signal' AND risk_signal_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(signal_id)
    .fetch_one(pool)
    .await
    .expect("action count should load")
}

pub(crate) async fn risk_audit_count(pool: &PgPool, tenant_id: Uuid, actor_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'risk.signal.resolved'
           AND target_type = 'billing_risk_signal'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}
