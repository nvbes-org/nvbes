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
async fn grant_operator_role_requires_strong_confirmation_and_active_actor_grant() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !operator_grant_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let tenant_id = seed_operator_grant_principals(&pool, actor_id, target_id, true).await;
    let app = Router::new()
        .merge(crate::identity_governance_operator_grant_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let generic_confirmation = app
        .clone()
        .oneshot(grant_request(
            target_id,
            actor_id,
            "platform_admin",
            "GRANT OPERATOR VIEWER",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let accepted = app
        .oneshot(grant_request(
            target_id,
            actor_id,
            "platform_admin",
            &crate::backoffice_authorization::strong_confirmation_code(
                "GRANT OPERATOR VIEWER",
                target_id,
            ),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["principal_id"], json!(target_id.to_string()));
    assert_eq!(payload["role"], json!("viewer"));
    assert_eq!(payload["next_status"], json!("active"));

    let stored_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM internal_admin_operator_grants WHERE principal_id = $1 AND role = 'viewer'",
    )
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("operator grant should exist");
    assert_eq!(stored_status, "active");

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'internal_admin.identity_governance.operator_grant.granted'
           AND target_type = 'operator_grant'
           AND target_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn grant_operator_role_rejects_header_role_without_active_actor_grant() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !operator_grant_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    seed_operator_grant_principals(&pool, actor_id, target_id, false).await;
    let app = Router::new()
        .merge(crate::identity_governance_operator_grant_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let denied = app
        .oneshot(grant_request(
            target_id,
            actor_id,
            "platform_admin",
            &crate::backoffice_authorization::strong_confirmation_code(
                "GRANT OPERATOR VIEWER",
                target_id,
            ),
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
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

async fn operator_grant_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.internal_admin_operator_grants') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_operator_grant_principals(
    pool: &PgPool,
    actor_id: Uuid,
    target_id: Uuid,
    grant_actor: bool,
) -> Uuid {
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
         VALUES ($1, 'enterprise', 'Operator Grant Tenant', $2, 'active', 'standard')",
    )
    .bind(tenant_id)
    .bind(format!("operator-grant-tenant-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("tenant should insert");

    for (id, display_name) in [
        (actor_id, "Backoffice Actor"),
        (target_id, "Target Operator"),
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

    for factor_type in ["webauthn", "totp"] {
        sqlx::query(
            "INSERT INTO mfa_factors (principal_id, factor_type, status, confirmed_at)
             VALUES ($1, $2::mfa_factor_type, 'active', NOW())",
        )
        .bind(target_id)
        .bind(factor_type)
        .execute(pool)
        .await
        .expect("target operator factor should insert");
    }

    if grant_actor {
        sqlx::query(
            "INSERT INTO internal_admin_operator_grants (
              principal_id, role, status, granted_by_principal_id, reason
            ) VALUES ($1, 'platform_admin', 'active', $1, 'test bootstrap')",
        )
        .bind(actor_id)
        .execute(pool)
        .await
        .expect("actor grant should insert");
    }

    tenant_id
}

fn grant_request(
    principal_id: Uuid,
    actor_id: Uuid,
    role: &str,
    confirm_code: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/admin/identity-governance-center/operator-grants/{principal_id}/viewer/grant"
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
                "reason": "ticket IAM-123 approved"
            })
            .to_string(),
        ))
        .expect("request should build")
}
