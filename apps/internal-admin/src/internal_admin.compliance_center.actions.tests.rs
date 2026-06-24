use axum::{
    Router,
    body::{self, Body},
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn erasure_request_rejects_generic_confirmation_code() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_compliance_confirmation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::compliance_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let response = app
        .oneshot(erasure_request(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "security_admin",
            "REQUEST ERASURE",
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn revoke_consent_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !compliance_actions_schema_exists(&pool).await {
        eprintln!("skipping test: compliance action schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let (tenant_id, workspace_id, consent_id) =
        seed_workspace_actor_principal_and_consent(&pool, actor_id, principal_id).await;
    let app = Router::new()
        .merge(crate::compliance_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "viewer",
            consent_id,
            "REVOKE CONSENT",
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "security_admin",
            consent_id,
            "REVOKE",
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "security_admin",
            consent_id,
            "REVOKE CONSENT",
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("revoke_consent"));
    assert_eq!(payload["status"], json!("applied"));

    let revoked = sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM user_consents WHERE id = $1",
    )
    .bind(consent_id)
    .fetch_one(&pool)
    .await
    .expect("consent status should load");
    assert!(revoked);

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_compliance_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'revoke_consent' AND consent_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(consent_id)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'compliance.consent.revoked'
           AND target_type = 'user_consent'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);

    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'compliance.consent.revoked'
           AND target_type = 'user_consent'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["compliance_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["object_links"]["consent_id"],
        json!(consent_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "revoked_at",
            "before": null,
            "after": "recorded"
        })
    );
}

async fn test_pool() -> Option<PgPool> {
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

async fn compliance_actions_schema_exists(pool: &PgPool) -> bool {
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

async fn seed_workspace_actor_principal_and_consent(
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

fn erasure_request(
    workspace_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/compliance/principals/{principal_id}/erasure-request"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": "ticket GDPR-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}

fn revoke_consent_request(
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
