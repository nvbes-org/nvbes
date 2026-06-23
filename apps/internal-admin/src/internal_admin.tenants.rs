use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_confirmation, require_idempotency_key, require_permission,
};
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct TenantDetail {
    id: Uuid,
    name: String,
    slug: String,
    status: String,
    kind: String,
    security_tier: String,
    workspace_count: i64,
    user_count: i64,
    audit_events_24h: i64,
    open_invoice_count: i64,
    provider_failure_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct TenantLifecycleRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct TenantLifecycleResult {
    tenant_id: Uuid,
    previous_status: String,
    next_status: String,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/tenants/{tenantId}", get(tenant_detail_route))
        .route(
            "/admin/tenants/{tenantId}/suspend",
            post(suspend_tenant_route),
        )
        .route(
            "/admin/tenants/{tenantId}/reactivate",
            post(reactivate_tenant_route),
        )
}

async fn tenant_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_tenant_detail(&state.db, tenant_id).await?))
}

async fn suspend_tenant_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<TenantLifecycleRequest>,
) -> Result<Json<TenantLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::TenantLifecycle)?;
    require_confirmation(&request.confirm_code, "SUSPEND TENANT")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_tenant_status(
            &state.db,
            actor_id,
            tenant_id,
            "suspended",
            "internal_admin.tenant.suspend",
            request,
        )
        .await?,
    ))
}

async fn reactivate_tenant_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<TenantLifecycleRequest>,
) -> Result<Json<TenantLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::TenantLifecycle)?;
    require_confirmation(&request.confirm_code, "REACTIVATE TENANT")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_tenant_status(
            &state.db,
            actor_id,
            tenant_id,
            "active",
            "internal_admin.tenant.reactivate",
            request,
        )
        .await?,
    ))
}

async fn load_tenant_detail(db: &PgPool, tenant_id: Uuid) -> Result<TenantDetail, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          t.id, t.name, t.slug, t.status::text, t.kind::text, t.security_tier,
          t.created_at, t.updated_at,
          (SELECT COUNT(*) FROM workspaces w WHERE w.tenant_id = t.id) AS workspace_count,
          (
            SELECT COUNT(*) FROM users u
            JOIN principals p ON p.id = u.principal_id
            WHERE p.tenant_id = t.id
          ) AS user_count,
          (
            SELECT COUNT(*) FROM audit_events ae
            WHERE ae.tenant_id = t.id AND ae.created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h,
          (
            SELECT COUNT(*) FROM billing_invoices bi
            WHERE bi.tenant_id = t.id AND bi.status::text IN ('issued', 'pro_forma')
          ) AS open_invoice_count,
          (
            SELECT COUNT(*) FROM billing_provider_events bpe
            WHERE bpe.tenant_id = t.id AND bpe.status::text IN ('failed', 'rejected')
          ) AS provider_failure_count
        FROM tenants t
        WHERE t.id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?;

    Ok(TenantDetail {
        id: row.get(0),
        name: row.get(1),
        slug: row.get(2),
        status: row.get(3),
        kind: row.get(4),
        security_tier: row.get(5),
        created_at: row.get(6),
        updated_at: row.get(7),
        workspace_count: row.get(8),
        user_count: row.get(9),
        audit_events_24h: row.get(10),
        open_invoice_count: row.get(11),
        provider_failure_count: row.get(12),
    })
}

async fn change_tenant_status(
    db: &PgPool,
    actor_id: Uuid,
    tenant_id: Uuid,
    next_status: &'static str,
    audit_action: &'static str,
    request: TenantLifecycleRequest,
) -> Result<TenantLifecycleResult, AppError> {
    validate_lifecycle_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let previous_status = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM tenants WHERE id = $1 FOR UPDATE",
    )
    .bind(tenant_id)
    .fetch_one(tx.as_mut())
    .await?;

    validate_tenant_status_transition(&previous_status, next_status)?;

    sqlx::query("UPDATE tenants SET status = $2::tenant_status, updated_at = NOW() WHERE id = $1")
        .bind(tenant_id)
        .bind(next_status)
        .execute(tx.as_mut())
        .await?;

    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'tenant', $1,
           jsonb_build_object('reason', $4, 'previous_status', $5, 'next_status', $6),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(audit_action)
    .bind(request.reason.trim())
    .bind(&previous_status)
    .bind(next_status)
    .execute(tx.as_mut())
    .await?;

    tx.commit().await?;

    Ok(TenantLifecycleResult {
        tenant_id,
        previous_status,
        next_status: next_status.to_string(),
        audit_action,
    })
}

fn validate_lifecycle_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Tenant lifecycle actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

fn validate_tenant_status_transition(
    previous_status: &str,
    next_status: &'static str,
) -> Result<(), AppError> {
    if previous_status == "deleted" {
        return Err(AppError::bad_request(
            "tenant_deleted",
            "Deleted tenants cannot be mutated from the back-office.",
        ));
    }
    if previous_status == next_status {
        return Err(AppError::bad_request(
            "tenant_status_unchanged",
            "Tenant is already in the requested status.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::{self, Body},
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
    use tower::ServiceExt;

    #[test]
    fn lifecycle_reason_must_be_detailed() {
        assert!(validate_lifecycle_reason("too short").is_err());
        assert!(validate_lifecycle_reason("ticket SEC-123 approved").is_ok());
    }

    #[test]
    fn tenant_lifecycle_rejects_deleted_and_unchanged_statuses() {
        assert!(validate_tenant_status_transition("deleted", "active").is_err());
        assert!(validate_tenant_status_transition("active", "active").is_err());
        assert!(validate_tenant_status_transition("active", "suspended").is_ok());
    }

    #[tokio::test]
    async fn suspend_tenant_route_enforces_role_confirmation_and_audits_success() {
        let Some(pool) = test_pool().await else {
            eprintln!("skipping test: Postgres is not reachable");
            return;
        };
        if !tenant_lifecycle_schema_exists(&pool).await {
            eprintln!("skipping test: tenant lifecycle schema is missing");
            return;
        }

        let actor_id = Uuid::new_v4();
        let tenant_id = seed_tenant_with_actor(&pool, actor_id).await;
        let app = Router::new()
            .merge(router())
            .with_state(crate::app::AppState::new(
                nvbes_core::config::AppConfig::default(),
                pool.clone(),
            ));

        let denied = app
            .clone()
            .oneshot(suspend_request(
                tenant_id,
                actor_id,
                "viewer",
                "SUSPEND TENANT",
                "ticket SEC-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let wrong_confirmation = app
            .clone()
            .oneshot(suspend_request(
                tenant_id,
                actor_id,
                "platform_admin",
                "SUSPEND",
                "ticket SEC-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

        let accepted = app
            .oneshot(suspend_request(
                tenant_id,
                actor_id,
                "platform_admin",
                "SUSPEND TENANT",
                "ticket SEC-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(accepted.status(), StatusCode::OK);
        let body = body::to_bytes(accepted.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["next_status"], json!("suspended"));

        let status =
            sqlx::query_scalar::<_, String>("SELECT status::text FROM tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_one(&pool)
                .await
                .expect("tenant should exist");
        assert_eq!(status, "suspended");

        let audit_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM audit_events
             WHERE tenant_id = $1 AND actor_principal_id = $2
               AND action = 'internal_admin.tenant.suspend'
               AND target_type = 'tenant'
               AND target_id = $1",
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

    async fn tenant_lifecycle_schema_exists(pool: &PgPool) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.tenants') IS NOT NULL
              AND to_regclass('public.principals') IS NOT NULL
              AND to_regclass('public.audit_events') IS NOT NULL",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
    }

    async fn seed_tenant_with_actor(pool: &PgPool, actor_id: Uuid) -> Uuid {
        let tenant_id = Uuid::new_v4();
        let slug = format!("test-tenant-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
             VALUES ($1, 'enterprise', 'Test Tenant', $2, 'active', 'standard')",
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

        tenant_id
    }

    fn suspend_request(
        tenant_id: Uuid,
        actor_id: Uuid,
        role: &str,
        confirm_code: &str,
        reason: &str,
    ) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!("/admin/tenants/{tenant_id}/suspend"))
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
}
