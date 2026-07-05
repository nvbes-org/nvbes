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

pub(crate) async fn compliance_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.user_consents') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.internal_admin_compliance_actions') IS NOT NULL",
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

pub(crate) async fn seed_workspace_actor_principal_and_consent(
    pool: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Compliance Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("compliance-tenant-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("tenant should insert");

    for (id, display_name) in [
        (actor_id, "Backoffice Actor"),
        (principal_id, "Data Subject"),
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
         VALUES ($1, $2, 'Compliance Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("compliance-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    let consent_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO user_consents (principal_id, document_version, consent_type)
         VALUES ($1, 'privacy-v1', 'privacy_policy') RETURNING id",
    )
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("consent should insert");

    (tenant_id, workspace_id, consent_id)
}

pub(crate) fn revoke_consent_request(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    consent_id: Uuid,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    revoke_consent_request_with_key(
        workspace_id,
        actor_id,
        role,
        consent_id,
        confirm_code,
        reason,
        &format!("test-{}", Uuid::new_v4()),
    )
}

pub(crate) fn revoke_consent_request_with_key(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    consent_id: Uuid,
    confirm_code: &str,
    reason: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/compliance/consents/{consent_id}/revoke"
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

pub(crate) fn revoke_consent_request_without_second_approver(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    consent_id: Uuid,
    confirm_code: &str,
    reason: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/compliance/consents/{consent_id}/revoke"
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

pub(crate) async fn compliance_action_count(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    consent_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_compliance_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'revoke_consent' AND consent_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(consent_id)
    .fetch_one(pool)
    .await
    .expect("action count should load")
}

pub(crate) async fn compliance_revoke_audit_count(
    pool: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'compliance.consent.revoked'
           AND target_type = 'user_consent'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}
