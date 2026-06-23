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

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/tenants/{tenantId}", get(tenant_detail_route))
}

async fn tenant_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_tenant_detail(&state.db, tenant_id).await?))
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
