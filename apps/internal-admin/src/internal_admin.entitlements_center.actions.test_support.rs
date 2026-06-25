use axum::{body, body::Body, http::Request};
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

pub(crate) async fn entitlement_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.internal_admin_entitlement_actions') IS NOT NULL",
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
    let slug = format!("entitlement-tenant-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Entitlement Tenant', $2, 'active', 'standard')",
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

    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, name, slug, workspace_type, plan_code)
         VALUES ($1, $2, 'Entitlement Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("entitlement-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    (tenant_id, workspace_id)
}

pub(crate) fn grant_request(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    grant_request_with_key(
        workspace_id,
        actor_id,
        role,
        confirm_code,
        reason,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn grant_request_with_key(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    reason: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/entitlements/grants"
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
                "feature_code": "advanced_search",
                "value": { "enabled": true },
                "reason": reason
            })
            .to_string(),
        ))
        .expect("request should build")
}

pub(crate) async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    serde_json::from_slice(&body).expect("body should be json")
}

pub(crate) async fn grant_action_count(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_entitlement_actions
         WHERE tenant_id = $1 AND workspace_id = $2
           AND actor_principal_id = $3 AND action_kind = 'grant_feature'
           AND feature_code = 'advanced_search'",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("action count should load")
}

pub(crate) async fn grant_audit_count(pool: &PgPool, tenant_id: Uuid, actor_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'entitlements.feature.granted'
           AND target_type = 'internal_admin_entitlement_action'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}

pub(crate) async fn assert_grant_audit_metadata(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    payload: &serde_json::Value,
) {
    assert_eq!(grant_audit_count(pool, tenant_id, actor_id).await, 1);
    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'entitlements.feature.granted'
           AND target_type = 'internal_admin_entitlement_action'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(
        audit_metadata["object_links"]["entitlement_action_id"],
        payload["object_id"]
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "feature_grant",
            "before": null,
            "after": "advanced_search"
        })
    );
}
