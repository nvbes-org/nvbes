use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1 as billing_pb;

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
    let actor_principal_id = actor_principal_id(&headers)?;
    let snapshot = crate::billing_grpc::get_admin_usage_center(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await?;
    Ok(Json(usage_center_from_grpc(snapshot)?))
}

fn usage_center_from_grpc(
    value: billing_pb::AdminUsageCenterSnapshot,
) -> Result<UsageCenterSnapshot, AppError> {
    Ok(UsageCenterSnapshot {
        active_meter_count: value.active_meter_count,
        usage_event_count_24h: value.usage_event_count_24h,
        usage_quantity_24h: value.usage_quantity_24h,
        correction_count_30d: value.correction_count_30d,
        rollup_count_current_period: value.rollup_count_current_period,
        distinct_tenant_count_24h: value.distinct_tenant_count_24h,
        meter_usage_24h: value
            .meter_usage_24h
            .into_iter()
            .map(meter_usage_from_grpc)
            .collect(),
        tenant_usage_24h: value
            .tenant_usage_24h
            .into_iter()
            .map(tenant_usage_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        recent_rollups: value
            .recent_rollups
            .into_iter()
            .map(rollup_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        recent_corrections: value
            .recent_corrections
            .into_iter()
            .map(correction_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn meter_usage_from_grpc(value: billing_pb::AdminMeterUsage) -> MeterUsage {
    MeterUsage {
        meter_code: value.meter_code,
        unit: value.unit,
        event_count: value.event_count,
        quantity: value.quantity,
    }
}

fn tenant_usage_from_grpc(value: billing_pb::AdminTenantUsage) -> Result<TenantUsage, AppError> {
    Ok(TenantUsage {
        tenant_id: parse_uuid(&value.tenant_id, "tenant usage tenant_id")?,
        tenant_name: value.tenant_name,
        event_count: value.event_count,
        quantity: value.quantity,
    })
}

fn rollup_from_grpc(value: billing_pb::AdminUsageRollup) -> Result<UsageRollup, AppError> {
    Ok(UsageRollup {
        id: parse_uuid(&value.id, "usage rollup id")?,
        tenant_id: parse_uuid(&value.tenant_id, "usage rollup tenant_id")?,
        tenant_name: value.tenant_name,
        workspace_id: parse_optional_uuid(&value.workspace_id, "usage rollup workspace_id")?,
        workspace_name: empty_to_none(value.workspace_name),
        meter_code: value.meter_code,
        quantity: value.quantity,
        unit: value.unit,
        period_start: parse_date(&value.period_start, "usage rollup period_start")?,
        period_end: parse_date(&value.period_end, "usage rollup period_end")?,
    })
}

fn correction_from_grpc(
    value: billing_pb::AdminUsageCorrection,
) -> Result<UsageCorrection, AppError> {
    Ok(UsageCorrection {
        id: parse_uuid(&value.id, "usage correction id")?,
        tenant_id: parse_uuid(&value.tenant_id, "usage correction tenant_id")?,
        tenant_name: value.tenant_name,
        meter_code: value.meter_code,
        quantity_delta: value.quantity_delta,
        reason: value.reason,
        created_by_principal_id: parse_optional_uuid(
            &value.created_by_principal_id,
            "usage correction actor",
        )?,
        created_at: parse_datetime(&value.created_at, "usage correction created_at")?,
    })
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
