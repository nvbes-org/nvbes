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
async fn reject_kyc_profile_rejects_generic_confirmation_code() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_billing_platform_confirmation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let response = app
        .oneshot(reject_request(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "finance_admin",
            "REJECT KYC",
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn disable_routing_rule_rejects_generic_confirmation_code() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_billing_platform_confirmation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let response = app
        .oneshot(disable_routing_request(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "finance_admin",
            "DISABLE ROUTING RULE",
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn approve_kyc_profile_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !billing_platform_actions_schema_exists(&pool).await {
        eprintln!("skipping test: billing platform action schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id, profile_id) = seed_kyc_profile_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code("APPROVE KYC", profile_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            "APPROVE",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            &crate::backoffice_authorization::strong_confirmation_code("APPROVE KYC", profile_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("approve_kyc_profile"));
    assert_eq!(payload["status"], json!("approved"));

    let status = sqlx::query_scalar::<_, String>(
        "SELECT review_status FROM billing_kyc_profiles WHERE id = $1",
    )
    .bind(profile_id)
    .fetch_one(&pool)
    .await
    .expect("kyc profile should load");
    assert_eq!(status, "approved");

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_billing_platform_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'approve_kyc_profile' AND kyc_profile_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(profile_id)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'billing_platform.kyc.approved'
           AND target_type = 'billing_kyc_profile'",
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
           AND action = 'billing_platform.kyc.approved'
           AND target_type = 'billing_kyc_profile'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(
        audit_metadata["billing_platform_action_id"],
        payload["object_id"]
    );
    assert_eq!(
        audit_metadata["object_links"]["kyc_profile_id"],
        json!(profile_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "review_status",
            "before": "pending",
            "after": "approved"
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

async fn billing_platform_actions_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND to_regclass('public.billing_kyc_profiles') IS NOT NULL
          AND to_regclass('public.internal_admin_billing_platform_actions') IS NOT NULL
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

async fn seed_kyc_profile_with_actor(pool: &PgPool, actor_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let profile_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Platform Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("platform-tenant-{}", Uuid::new_v4()))
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
         VALUES ($1, $2, 'Platform Workspace', $3, 'team', 'enterprise')",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("platform-workspace-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("workspace should insert");

    sqlx::query(
        "INSERT INTO billing_kyc_profiles (
           id, tenant_id, company_name, proof_reference, review_status
         ) VALUES ($1, $2, 'Acme Platform', 'proof://kyc', 'pending')",
    )
    .bind(profile_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("kyc profile should insert");

    (tenant_id, workspace_id, profile_id)
}

fn reject_request(
    workspace_id: Uuid,
    profile_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/billing-platform/kyc-profiles/{profile_id}/reject"
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
                "reason": "ticket BPL-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}

fn disable_routing_request(
    workspace_id: Uuid,
    rule_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/billing-platform/routing-rules/{rule_id}/disable"
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
                "reason": "ticket BPL-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}

fn approve_request(
    workspace_id: Uuid,
    profile_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/workspaces/{workspace_id}/admin/billing-platform/kyc-profiles/{profile_id}/approve"
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
                "reason": "ticket BPL-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}
