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
    BackofficePermission, require_idempotency_key, require_operator_role_grant, require_permission,
    require_strong_confirmation,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
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
    let actor_principal_id = actor_principal_id(&headers)?;
    Ok(Json(
        load_tenant_detail(
            &state.db,
            &state.billing_grpc_endpoint,
            actor_principal_id,
            tenant_id,
        )
        .await?,
    ))
}

async fn suspend_tenant_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<TenantLifecycleRequest>,
) -> Result<Json<TenantLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::TenantLifecycle)?;
    require_operator_role_grant(&state.db, &headers).await?;
    require_strong_confirmation(&request.confirm_code, "SUSPEND TENANT", tenant_id)?;
    require_dual_control(&headers)?;
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
    require_operator_role_grant(&state.db, &headers).await?;
    require_strong_confirmation(&request.confirm_code, "REACTIVATE TENANT", tenant_id)?;
    require_dual_control(&headers)?;
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

async fn load_tenant_detail(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    actor_principal_id: Uuid,
    tenant_id: Uuid,
) -> Result<TenantDetail, AppError> {
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
          ) AS audit_events_24h
        FROM tenants t
        WHERE t.id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?;
    let billing_summary = crate::billing_grpc::get_admin_tenant_billing_summary(
        billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id,
            actor_principal_id,
        },
        tenant_id,
    )
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
        open_invoice_count: billing_summary.open_invoice_count,
        provider_failure_count: billing_summary.provider_failure_count,
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
#[path = "internal_admin.tenants.tests.rs"]
mod tests;
