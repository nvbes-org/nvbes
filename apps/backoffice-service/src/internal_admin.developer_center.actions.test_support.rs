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

pub(crate) async fn developer_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.oauth_clients') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.internal_admin_developer_actions') IS NOT NULL",
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

pub(crate) async fn seed_workspace_actor_and_client(
    pool: &PgPool,
    actor_id: Uuid,
    client_id: &str,
) -> (Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Developer Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("developer-tenant-{}", Uuid::new_v4()))
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
         VALUES ($1, $2, 'Developer Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("developer-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    sqlx::query(
        "INSERT INTO oauth_clients (
           tenant_id, client_id, client_secret_hash, name, redirect_uris,
           owner_scope_type, owner_scope_id, client_type
         ) VALUES ($1, $2, 'hash', 'Backoffice Test Client', ARRAY['https://example.com/callback'],
           'tenant', $1, 'confidential')",
    )
    .bind(tenant_id)
    .bind(client_id)
    .execute(pool)
    .await
    .expect("client should insert");

    (tenant_id, workspace_id)
}

pub(crate) fn revoke_request(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    client_id: &str,
    reason: &str,
) -> Request<Body> {
    revoke_request_with_key(
        workspace_id,
        actor_id,
        role,
        confirm_code,
        client_id,
        reason,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn revoke_request_with_key(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    client_id: &str,
    reason: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/developer/clients/{client_id}/revoke"
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

pub(crate) async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    serde_json::from_slice(&body).expect("body should be json")
}

pub(crate) async fn client_is_revoked(pool: &PgPool, tenant_id: Uuid, client_id: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM oauth_clients
         WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_one(pool)
    .await
    .expect("client status should load")
}

pub(crate) async fn developer_action_count(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    client_id: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_developer_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'revoke_client' AND client_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(client_id)
    .fetch_one(pool)
    .await
    .expect("action count should load")
}

pub(crate) async fn developer_audit_count(pool: &PgPool, tenant_id: Uuid, actor_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'developer.client.revoked'
           AND target_type = 'oauth_client'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}

pub(crate) async fn assert_developer_audit_metadata(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    payload: &serde_json::Value,
) {
    assert_eq!(developer_audit_count(pool, tenant_id, actor_id).await, 1);
    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'developer.client.revoked'
           AND target_type = 'oauth_client'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["developer_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "revoked_at",
            "before": null,
            "after": "recorded"
        })
    );
}
