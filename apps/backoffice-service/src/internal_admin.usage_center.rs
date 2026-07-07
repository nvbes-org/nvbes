use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct UsageCenterSnapshot {
    active_meter_count: i64,
    usage_event_count_24h: i64,
    usage_quantity_24h: i64,
    correction_count_30d: i64,
    rollup_count_current_period: i64,
    distinct_tenant_count_24h: i64,
    meter_usage_24h: Vec<MeterUsage>,
    tenant_usage_24h: Vec<TenantUsage>,
    recent_rollups: Vec<UsageRollup>,
    recent_corrections: Vec<UsageCorrection>,
}

#[derive(Debug, Serialize)]
struct MeterUsage {
    meter_code: String,
    unit: String,
    event_count: i64,
    quantity: i64,
}

#[derive(Debug, Serialize)]
struct TenantUsage {
    tenant_id: Uuid,
    tenant_name: String,
    event_count: i64,
    quantity: i64,
}

#[derive(Debug, Serialize)]
struct UsageRollup {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    meter_code: String,
    quantity: i64,
    unit: String,
    period_start: NaiveDate,
    period_end: NaiveDate,
}

#[derive(Debug, Serialize)]
struct UsageCorrection {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    meter_code: String,
    quantity_delta: i64,
    reason: String,
    created_by_principal_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/usage-center", get(usage_center_route))
}

async fn usage_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UsageCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_usage_center(&state.db).await?))
}

async fn load_usage_center(db: &PgPool) -> Result<UsageCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM billing_meter_definitions WHERE status = 'active')
            AS active_meter_count,
          (
            SELECT COUNT(*) FROM billing_usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS usage_event_count_24h,
          (
            SELECT COALESCE(SUM(quantity), 0) FROM billing_usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS usage_quantity_24h,
          (
            SELECT COUNT(*) FROM billing_usage_corrections
            WHERE created_at >= NOW() - INTERVAL '30 days'
          ) AS correction_count_30d,
          (
            SELECT COUNT(*) FROM billing_usage_rollups
            WHERE period_end >= CURRENT_DATE
          ) AS rollup_count_current_period,
          (
            SELECT COUNT(DISTINCT tenant_id) FROM billing_usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS distinct_tenant_count_24h
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(UsageCenterSnapshot {
        active_meter_count: metrics.get("active_meter_count"),
        usage_event_count_24h: metrics.get("usage_event_count_24h"),
        usage_quantity_24h: metrics.get("usage_quantity_24h"),
        correction_count_30d: metrics.get("correction_count_30d"),
        rollup_count_current_period: metrics.get("rollup_count_current_period"),
        distinct_tenant_count_24h: metrics.get("distinct_tenant_count_24h"),
        meter_usage_24h: load_meter_usage(db).await?,
        tenant_usage_24h: load_tenant_usage(db).await?,
        recent_rollups: load_recent_rollups(db).await?,
        recent_corrections: load_recent_corrections(db).await?,
    })
}

async fn load_meter_usage(db: &PgPool) -> Result<Vec<MeterUsage>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT meter_code, unit, COUNT(*) AS event_count, COALESCE(SUM(quantity), 0) AS quantity
        FROM billing_usage_events
        WHERE occurred_at >= NOW() - INTERVAL '24 hours'
        GROUP BY meter_code, unit
        ORDER BY quantity DESC, event_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MeterUsage {
            meter_code: row.get("meter_code"),
            unit: row.get("unit"),
            event_count: row.get("event_count"),
            quantity: row.get("quantity"),
        })
        .collect())
}

async fn load_tenant_usage(db: &PgPool) -> Result<Vec<TenantUsage>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ue.tenant_id, t.name AS tenant_name, COUNT(*) AS event_count,
          COALESCE(SUM(ue.quantity), 0) AS quantity
        FROM billing_usage_events ue
        JOIN tenants t ON t.id = ue.tenant_id
        WHERE ue.occurred_at >= NOW() - INTERVAL '24 hours'
        GROUP BY ue.tenant_id, t.name
        ORDER BY quantity DESC, event_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| TenantUsage {
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            event_count: row.get("event_count"),
            quantity: row.get("quantity"),
        })
        .collect())
}

async fn load_recent_rollups(db: &PgPool) -> Result<Vec<UsageRollup>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ur.id, ur.tenant_id, t.name AS tenant_name, ur.workspace_id,
          w.name AS workspace_name, ur.meter_code, ur.quantity, ur.unit,
          ur.period_start, ur.period_end
        FROM billing_usage_rollups ur
        JOIN tenants t ON t.id = ur.tenant_id
        LEFT JOIN workspaces w ON w.id = ur.workspace_id
        ORDER BY ur.updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UsageRollup {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: row.get("workspace_id"),
            workspace_name: row.get("workspace_name"),
            meter_code: row.get("meter_code"),
            quantity: row.get("quantity"),
            unit: row.get("unit"),
            period_start: row.get("period_start"),
            period_end: row.get("period_end"),
        })
        .collect())
}

async fn load_recent_corrections(db: &PgPool) -> Result<Vec<UsageCorrection>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT uc.id, uc.tenant_id, t.name AS tenant_name, uc.meter_code,
          uc.quantity_delta, uc.reason, uc.created_by_principal_id, uc.created_at
        FROM billing_usage_corrections uc
        JOIN tenants t ON t.id = uc.tenant_id
        ORDER BY uc.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UsageCorrection {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            meter_code: row.get("meter_code"),
            quantity_delta: row.get("quantity_delta"),
            reason: row.get("reason"),
            created_by_principal_id: row.get("created_by_principal_id"),
            created_at: row.get("created_at"),
        })
        .collect())
}
