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
async fn pending_approvals_lists_work_and_marketplace_action_uses_returned_target() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !pending_approvals_schema_exists(&pool).await {
        eprintln!("skipping test: pending approval schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let seed = seed_pending_approval_sources(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::pending_approvals::router())
        .merge(crate::developer_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let listed = app
        .clone()
        .oneshot(list_request(actor_id))
        .await
        .expect("route should respond");
    assert_eq!(listed.status(), StatusCode::OK);
    let body = body::to_bytes(listed.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["pending_count"], json!(3));
    assert_eq!(payload["critical_count"], json!(1));
    assert_eq!(payload["overdue_count"], json!(1));

    let marketplace = payload["items"]
        .as_array()
        .expect("items should be an array")
        .iter()
        .find(|item| item["target_type"] == json!("marketplace_app"))
        .expect("marketplace item should be present");
    assert_eq!(marketplace["target_id"], json!(seed.marketplace_app_id));
    assert_eq!(marketplace["audit_hint"], json!("APPROVE MARKETPLACE APP"));

    let recovery = payload["items"]
        .as_array()
        .expect("items should be an array")
        .iter()
        .find(|item| item["target_type"] == json!("recovery_request"))
        .expect("recovery item should be present");
    assert_eq!(recovery["target_id"], json!(seed.recovery_request_id));
    assert_eq!(recovery["audit_hint"], json!("CANCEL RECOVERY"));

    let accepted = app
        .oneshot(approve_marketplace_request(
            seed.workspace_id,
            seed.marketplace_app_id,
            actor_id,
            &crate::backoffice_authorization::strong_confirmation_code(
                "APPROVE MARKETPLACE APP",
                seed.marketplace_app_id,
            ),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);

    let approved = sqlx::query_scalar::<_, bool>(
        "SELECT status::text = 'approved' FROM developer_marketplace_apps WHERE id = $1",
    )
    .bind(seed.marketplace_app_id)
    .fetch_one(&pool)
    .await
    .expect("marketplace status should load");
    assert!(approved);
}

struct PendingApprovalSeed {
    marketplace_app_id: Uuid,
    recovery_request_id: Uuid,
    workspace_id: Uuid,
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

async fn pending_approvals_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.users') IS NOT NULL
          AND to_regclass('public.oauth_clients') IS NOT NULL
          AND to_regclass('public.developer_marketplace_apps') IS NOT NULL
          AND to_regclass('public.enterprise_password_recovery_requests') IS NOT NULL
          AND to_regclass('public.billing_kyc_profiles') IS NOT NULL
          AND to_regclass('public.internal_admin_developer_actions') IS NOT NULL
          AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'billing_kyc_profiles'
              AND column_name = 'review_status'
          )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_pending_approval_sources(pool: &PgPool, actor_id: Uuid) -> PendingApprovalSeed {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let marketplace_app_id = Uuid::new_v4();
    let recovery_request_id = Uuid::new_v4();
    let kyc_profile_id = Uuid::new_v4();
    let client_id = format!("client_{}", Uuid::new_v4());

    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Approvals Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("approvals-tenant-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("tenant should insert");
    for (id, name) in [
        (actor_id, "Backoffice Actor"),
        (principal_id, "Recovery User"),
    ] {
        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
             VALUES ($1, $2, 'human', 'active', $3)",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .execute(pool)
        .await
        .expect("principal should insert");
    }
    sqlx::query(
        "INSERT INTO users (principal_id, email, name, status, email_verified_at)
         VALUES ($1, 'recovery@example.test', 'Recovery User', 'active', NOW())",
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("user should insert");
    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, name, slug, workspace_type, plan_code)
         VALUES ($1, $2, 'Approvals Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("approvals-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");
    sqlx::query(
        "INSERT INTO oauth_clients (
           tenant_id, client_id, client_secret_hash, name, redirect_uris,
           owner_scope_type, owner_scope_id, client_type
         ) VALUES ($1, $2, 'hash', 'Approvals Client', ARRAY['https://example.com/callback'],
           'tenant', $1, 'confidential')",
    )
    .bind(tenant_id)
    .bind(&client_id)
    .execute(pool)
    .await
    .expect("client should insert");
    sqlx::query(
        "INSERT INTO developer_marketplace_apps (id, tenant_id, client_id, status, submitted_by)
         VALUES ($1, $2, $3, 'pending', $4)",
    )
    .bind(marketplace_app_id)
    .bind(tenant_id)
    .bind(&client_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("marketplace app should insert");
    sqlx::query(
        "INSERT INTO enterprise_password_recovery_requests (
           id, tenant_id, principal_id, email, status, available_at, created_at
         ) VALUES ($1, $2, $3, 'recovery@example.test', 'pending', NOW() - INTERVAL '1 hour', NOW() - INTERVAL '2 hours')",
    )
    .bind(recovery_request_id)
    .bind(tenant_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("recovery request should insert");
    sqlx::query(
        "INSERT INTO billing_kyc_profiles (
           id, tenant_id, company_name, proof_reference, review_status
         ) VALUES ($1, $2, 'Approvals KYC', 'proof://approvals', 'pending')",
    )
    .bind(kyc_profile_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("kyc profile should insert");

    PendingApprovalSeed {
        marketplace_app_id,
        recovery_request_id,
        workspace_id,
    }
}

fn list_request(actor_id: Uuid) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/admin/pending-approvals")
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .body(Body::empty())
        .expect("request should build")
}

fn approve_marketplace_request(
    workspace_id: Uuid,
    app_id: Uuid,
    actor_id: Uuid,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/developer/marketplace-apps/{app_id}/approve"
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
        .header("x-nvbes-actor-principal-id", actor_id.to_string())
        .header("x-nvbes-backoffice-role", "developer_admin")
        .header(
            "x-nvbes-second-approver-principal-id",
            Uuid::new_v4().to_string(),
        )
        .header("x-nvbes-second-approver-role", "platform_admin")
        .body(Body::from(
            json!({
                "confirm_code": confirm_code,
                "reason": "ticket APPROVAL-123 reviewed"
            })
            .to_string(),
        ))
        .expect("request should build")
}
