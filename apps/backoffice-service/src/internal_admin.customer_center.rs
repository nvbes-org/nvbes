use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct CustomerCenterSnapshot {
    active_tenant_count: i64,
    suspended_tenant_count: i64,
    dormant_workspace_count: i64,
    pending_invitation_count: i64,
    expired_invitation_count: i64,
    usage_events_24h: i64,
    storage_bytes_used: i64,
    file_count: i64,
    high_storage_workspaces: Vec<HighStorageWorkspace>,
    dormant_workspaces: Vec<DormantWorkspace>,
    tenants_with_pending_invites: Vec<TenantPendingInvites>,
}

#[derive(Debug, Serialize)]
struct HighStorageWorkspace {
    workspace_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_name: String,
    plan_code: String,
    used_storage_bytes: i64,
    file_count: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct DormantWorkspace {
    workspace_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_name: String,
    plan_code: String,
    last_usage_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct TenantPendingInvites {
    tenant_id: Uuid,
    tenant_name: String,
    pending_invitation_count: i64,
    oldest_invitation_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/customer-center", get(customer_center_route))
}

async fn customer_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CustomerCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_customer_center(&state.db).await?))
}

async fn load_customer_center(db: &PgPool) -> Result<CustomerCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM tenants WHERE status::text = 'active') AS active_tenant_count,
          (SELECT COUNT(*) FROM tenants WHERE status::text = 'suspended') AS suspended_tenant_count,
          (
            SELECT COUNT(*) FROM workspaces w
            WHERE NOT EXISTS (
              SELECT 1 FROM usage_events ue
              WHERE ue.workspace_id = w.id AND ue.occurred_at >= NOW() - INTERVAL '30 days'
            )
          ) AS dormant_workspace_count,
          (
            SELECT COUNT(*) FROM workspace_invitations
            WHERE status::text = 'pending' AND expires_at >= NOW()
          ) AS pending_invitation_count,
          (
            SELECT COUNT(*) FROM workspace_invitations
            WHERE status::text = 'pending' AND expires_at < NOW()
          ) AS expired_invitation_count,
          (
            SELECT COUNT(*) FROM usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS usage_events_24h,
          (SELECT COALESCE(SUM(used_storage_bytes), 0) FROM quota_usage) AS storage_bytes_used,
          (SELECT COALESCE(SUM(file_count), 0) FROM quota_usage) AS file_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(CustomerCenterSnapshot {
        active_tenant_count: metrics.get("active_tenant_count"),
        suspended_tenant_count: metrics.get("suspended_tenant_count"),
        dormant_workspace_count: metrics.get("dormant_workspace_count"),
        pending_invitation_count: metrics.get("pending_invitation_count"),
        expired_invitation_count: metrics.get("expired_invitation_count"),
        usage_events_24h: metrics.get("usage_events_24h"),
        storage_bytes_used: metrics.get("storage_bytes_used"),
        file_count: metrics.get("file_count"),
        high_storage_workspaces: load_high_storage_workspaces(db).await?,
        dormant_workspaces: load_dormant_workspaces(db).await?,
        tenants_with_pending_invites: load_tenants_with_pending_invites(db).await?,
    })
}

async fn load_high_storage_workspaces(db: &PgPool) -> Result<Vec<HighStorageWorkspace>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT w.id AS workspace_id, w.tenant_id, t.name AS tenant_name,
          w.name AS workspace_name, w.plan_code, qu.used_storage_bytes,
          qu.file_count, qu.updated_at
        FROM quota_usage qu
        JOIN workspaces w ON w.id = qu.workspace_id
        JOIN tenants t ON t.id = w.tenant_id
        ORDER BY qu.used_storage_bytes DESC, qu.file_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| HighStorageWorkspace {
            workspace_id: row.get("workspace_id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_name: row.get("workspace_name"),
            plan_code: row.get("plan_code"),
            used_storage_bytes: row.get("used_storage_bytes"),
            file_count: row.get("file_count"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_dormant_workspaces(db: &PgPool) -> Result<Vec<DormantWorkspace>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT w.id AS workspace_id, w.tenant_id, t.name AS tenant_name,
          w.name AS workspace_name, w.plan_code,
          (SELECT MAX(ue.occurred_at) FROM usage_events ue WHERE ue.workspace_id = w.id)
            AS last_usage_at,
          w.created_at
        FROM workspaces w
        JOIN tenants t ON t.id = w.tenant_id
        WHERE NOT EXISTS (
          SELECT 1 FROM usage_events ue
          WHERE ue.workspace_id = w.id AND ue.occurred_at >= NOW() - INTERVAL '30 days'
        )
        ORDER BY w.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| DormantWorkspace {
            workspace_id: row.get("workspace_id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_name: row.get("workspace_name"),
            plan_code: row.get("plan_code"),
            last_usage_at: row.get("last_usage_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_tenants_with_pending_invites(
    db: &PgPool,
) -> Result<Vec<TenantPendingInvites>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT t.id AS tenant_id, t.name AS tenant_name, COUNT(*) AS pending_invitation_count,
          MIN(wi.created_at) AS oldest_invitation_at
        FROM workspace_invitations wi
        JOIN workspaces w ON w.id = wi.workspace_id
        JOIN tenants t ON t.id = w.tenant_id
        WHERE wi.status::text = 'pending'
        GROUP BY t.id, t.name
        ORDER BY pending_invitation_count DESC, oldest_invitation_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| TenantPendingInvites {
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            pending_invitation_count: row.get("pending_invitation_count"),
            oldest_invitation_at: row.get("oldest_invitation_at"),
        })
        .collect())
}
