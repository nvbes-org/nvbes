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
async fn suppress_email_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !communications_actions_schema_exists(&pool).await {
        eprintln!("skipping test: communications action schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let email = format!("{}@example.com", Uuid::new_v4());
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::communications_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code(
                "SUPPRESS EMAIL",
                workspace_id,
            ),
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "support_agent",
            "SUPPRESS",
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "support_agent",
            "SUPPRESS EMAIL",
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "support_agent",
            &crate::backoffice_authorization::strong_confirmation_code(
                "SUPPRESS EMAIL",
                workspace_id,
            ),
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("suppress_email"));
    assert_eq!(payload["status"], json!("applied"));

    let suppressed_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM suppressed_emails WHERE email = $1")
            .bind(&email)
            .fetch_one(&pool)
            .await
            .expect("suppression count should load");
    assert_eq!(suppressed_count, 1);

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_communications_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'suppress_email' AND email = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'communications.email.suppressed'
           AND target_type = 'suppressed_email'",
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
           AND action = 'communications.email.suppressed'
           AND target_type = 'suppressed_email'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(
        audit_metadata["communications_action_id"],
        payload["object_id"]
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "suppression",
            "before": null,
            "after": "applied"
        })
    );
    let expected_redacted_email = format!("{}***@example.com", &email[..1]);
    assert_eq!(
        audit_metadata["changes"][1]["after"],
        json!(expected_redacted_email)
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

async fn communications_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.suppressed_emails') IS NOT NULL
          AND to_regclass('public.internal_admin_communications_actions') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_workspace_with_actor(pool: &PgPool, actor_id: Uuid) -> (Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Communications Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("communications-tenant-{}", Uuid::new_v4()))
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
         VALUES ($1, $2, 'Communications Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("communications-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    (tenant_id, workspace_id)
}

fn suppress_request(
    workspace_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
    email: &str,
    reason: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/communications/suppressions"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "email": email,
                "reason": reason
            })
            .to_string(),
        ))
        .expect("request should build")
}
