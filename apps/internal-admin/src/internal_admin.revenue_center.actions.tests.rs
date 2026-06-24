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
async fn hold_invoice_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !revenue_actions_schema_exists(&pool).await {
        eprintln!("skipping test: revenue action schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id, invoice_id) = seed_invoice_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::revenue_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "viewer",
            "HOLD INVOICE",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            "HOLD",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            "HOLD INVOICE",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("hold_invoice"));
    assert_eq!(payload["status"], json!("held"));

    let hold_actor = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT internal_hold_by_principal_id FROM billing_invoices WHERE id = $1",
    )
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("invoice hold actor should load");
    assert_eq!(hold_actor, Some(actor_id));

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_revenue_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'hold_invoice' AND invoice_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'revenue.invoice.held'
           AND target_type = 'billing_invoice'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);
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

async fn revenue_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.billing_invoices') IS NOT NULL
          AND to_regclass('public.internal_admin_revenue_actions') IS NOT NULL
          AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'billing_invoices'
              AND column_name = 'internal_hold_by_principal_id'
          )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_invoice_with_actor(pool: &PgPool, actor_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let invoice_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Revenue Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("revenue-tenant-{}", Uuid::new_v4()))
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
         VALUES ($1, $2, 'Revenue Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("revenue-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    sqlx::query(
        "INSERT INTO billing_invoices (
           id, tenant_id, invoice_number, status, currency, total_minor, issued_at, due_at
         ) VALUES ($1, $2, $3, 'issued', 'EUR', 12000, NOW(), NOW() - INTERVAL '1 day')",
    )
    .bind(invoice_id)
    .bind(tenant_id)
    .bind(format!("INV-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("invoice should insert");

    (tenant_id, workspace_id, invoice_id)
}

fn hold_request(
    workspace_id: Uuid,
    invoice_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/revenue/invoices/{invoice_id}/hold"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", role)
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": "ticket REV-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}
