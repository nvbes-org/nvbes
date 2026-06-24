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
async fn replay_provider_event_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !operations_actions_schema_exists(&pool).await {
        eprintln!("skipping test: operations action schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id, event_id) = seed_provider_event_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::operations_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(replay_request(
            workspace_id,
            event_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code(
                "REPLAY PROVIDER EVENT",
                event_id,
            ),
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(replay_request(
            workspace_id,
            event_id,
            actor_id,
            "security_admin",
            "REPLAY EVENT",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(replay_request(
            workspace_id,
            event_id,
            actor_id,
            "security_admin",
            "REPLAY PROVIDER EVENT",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(replay_request(
            workspace_id,
            event_id,
            actor_id,
            "security_admin",
            &crate::backoffice_authorization::strong_confirmation_code(
                "REPLAY PROVIDER EVENT",
                event_id,
            ),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("replay_provider_event"));
    assert_eq!(payload["status"], json!("received"));

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM billing_provider_events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_one(&pool)
    .await
    .expect("provider event should load");
    assert_eq!(status, "received");

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_operations_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'replay_provider_event' AND provider_event_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(event_id)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'operations.provider_event.replayed'
           AND target_type = 'billing_provider_event'",
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
           AND action = 'operations.provider_event.replayed'
           AND target_type = 'billing_provider_event'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["operations_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["object_links"]["provider_event_id"],
        json!(event_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "status",
            "before": "failed",
            "after": "received"
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

async fn operations_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.billing_provider_events') IS NOT NULL
          AND to_regclass('public.internal_admin_operations_actions') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_provider_event_with_actor(pool: &PgPool, actor_id: Uuid) -> (Uuid, Uuid, Uuid) {
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

fn replay_request(
    workspace_id: Uuid,
    event_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/operations/provider-events/{event_id}/replay"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": "ticket OPS-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}
