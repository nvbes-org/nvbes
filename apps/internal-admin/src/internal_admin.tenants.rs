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
    BackofficePermission, require_confirmation, require_permission,
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
}
