use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct WorkspaceDetail {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    name: String,
    workspace_type: String,
    plan_code: String,
    trial_ends_at: Option<DateTime<Utc>>,
    member_count: i64,
    owner_count: i64,
    active_member_count: i64,
    service_account_count: i64,
    audit_events_24h: i64,
    open_invoice_count: i64,
    active_subscription_count: i64,
    latest_audit_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/workspaces/{workspaceId}",
        get(workspace_detail_route),
    )
}

async fn workspace_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_workspace_detail(&state.db, workspace_id).await?))
}

async fn load_workspace_detail(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<WorkspaceDetail, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          w.id, w.tenant_id, t.name AS tenant_name, w.name,
          w.workspace_type::text, w.plan_code, w.trial_ends_at,
          w.created_at, w.updated_at,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.workspace_id = w.id
          ) AS member_count,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.workspace_id = w.id AND wm.role::text = 'owner'
          ) AS owner_count,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.workspace_id = w.id AND wm.status::text = 'active'
          ) AS active_member_count,
          (
            SELECT COUNT(*) FROM service_accounts sa
            WHERE sa.workspace_id = w.id
          ) AS service_account_count,
          (
            SELECT COUNT(*) FROM audit_events ae
            WHERE ae.workspace_id = w.id AND ae.created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h,
          (
            SELECT COUNT(DISTINCT bi.id)
            FROM billing_invoices bi
            LEFT JOIN billing_accounts ba ON ba.id = bi.billing_account_id
            LEFT JOIN billing_subscriptions bs ON bs.id = bi.subscription_id
            WHERE (ba.workspace_id = w.id OR bs.workspace_id = w.id)
              AND bi.status::text IN ('issued', 'pro_forma')
          ) AS open_invoice_count,
          (
            SELECT COUNT(*) FROM billing_subscriptions bs
            WHERE bs.workspace_id = w.id AND bs.status IN ('active', 'trialing')
          ) AS active_subscription_count,
          (
            SELECT MAX(ae.created_at) FROM audit_events ae
            WHERE ae.workspace_id = w.id
          ) AS latest_audit_at
        FROM workspaces w
        JOIN tenants t ON t.id = w.tenant_id
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(db)
    .await?;

    Ok(WorkspaceDetail {
        id: row.get(0),
        tenant_id: row.get(1),
        tenant_name: row.get(2),
        name: row.get(3),
        workspace_type: row.get(4),
        plan_code: row.get(5),
        trial_ends_at: row.get(6),
        created_at: row.get(7),
        updated_at: row.get(8),
        member_count: row.get(9),
        owner_count: row.get(10),
        active_member_count: row.get(11),
        service_account_count: row.get(12),
        audit_events_24h: row.get(13),
        open_invoice_count: row.get(14),
        active_subscription_count: row.get(15),
        latest_audit_at: row.get(16),
    })
}
