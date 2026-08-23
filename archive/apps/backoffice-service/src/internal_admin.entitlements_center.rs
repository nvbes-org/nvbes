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
struct EntitlementsSnapshot {
    active_plan_count: i64,
    active_feature_count: i64,
    quota_definition_count: i64,
    active_entitlement_count: i64,
    over_quota_balance_count: i64,
    unpublished_change_count: i64,
    active_trial_grant_count: i64,
    active_plans: Vec<EntitlementPlan>,
    over_quota_balances: Vec<OverQuotaBalance>,
    expiring_entitlements: Vec<ExpiringEntitlement>,
    unpublished_changes: Vec<UnpublishedEntitlementChange>,
}

#[derive(Debug, Serialize)]
struct EntitlementPlan {
    plan_id: Uuid,
    product_name: String,
    plan_code: String,
    plan_name: String,
    active_version_count: i64,
    feature_count: i64,
}

#[derive(Debug, Serialize)]
struct OverQuotaBalance {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    quota_code: String,
    included_quantity: i64,
    used_quantity: i64,
    period_end: NaiveDate,
}

#[derive(Debug, Serialize)]
struct ExpiringEntitlement {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    status: String,
    effective_to: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct UnpublishedEntitlementChange {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    event_id: String,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/entitlements-center", get(entitlements_center_route))
}

async fn entitlements_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<EntitlementsSnapshot>, AppError> {
    let actor_principal_id = actor_principal_id(&headers)?;
    let snapshot = crate::billing_grpc::get_admin_entitlements_center(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await?;
    Ok(Json(entitlements_from_grpc(snapshot)?))
}

fn entitlements_from_grpc(
    value: billing_pb::AdminEntitlementsCenterSnapshot,
) -> Result<EntitlementsSnapshot, AppError> {
    Ok(EntitlementsSnapshot {
        active_plan_count: value.active_plan_count,
        active_feature_count: value.active_feature_count,
        quota_definition_count: value.quota_definition_count,
        active_entitlement_count: value.active_entitlement_count,
        over_quota_balance_count: value.over_quota_balance_count,
        unpublished_change_count: value.unpublished_change_count,
        active_trial_grant_count: value.active_trial_grant_count,
        active_plans: value
            .active_plans
            .into_iter()
            .map(plan_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        over_quota_balances: value
            .over_quota_balances
            .into_iter()
            .map(over_quota_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        expiring_entitlements: value
            .expiring_entitlements
            .into_iter()
            .map(expiring_entitlement_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        unpublished_changes: value
            .unpublished_changes
            .into_iter()
            .map(unpublished_change_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn plan_from_grpc(value: billing_pb::AdminEntitlementPlan) -> Result<EntitlementPlan, AppError> {
    Ok(EntitlementPlan {
        plan_id: parse_uuid(&value.plan_id, "entitlement plan_id")?,
        product_name: value.product_name,
        plan_code: value.plan_code,
        plan_name: value.plan_name,
        active_version_count: value.active_version_count,
        feature_count: value.feature_count,
    })
}

fn over_quota_from_grpc(
    value: billing_pb::AdminOverQuotaBalance,
) -> Result<OverQuotaBalance, AppError> {
    Ok(OverQuotaBalance {
        id: parse_uuid(&value.id, "over quota balance id")?,
        tenant_id: parse_uuid(&value.tenant_id, "over quota balance tenant_id")?,
        tenant_name: value.tenant_name,
        workspace_id: parse_optional_uuid(&value.workspace_id, "over quota balance workspace_id")?,
        workspace_name: empty_to_none(value.workspace_name),
        quota_code: value.quota_code,
        included_quantity: value.included_quantity,
        used_quantity: value.used_quantity,
        period_end: parse_date(&value.period_end, "over quota balance period_end")?,
    })
}

fn expiring_entitlement_from_grpc(
    value: billing_pb::AdminExpiringEntitlement,
) -> Result<ExpiringEntitlement, AppError> {
    Ok(ExpiringEntitlement {
        id: parse_uuid(&value.id, "expiring entitlement id")?,
        tenant_id: parse_uuid(&value.tenant_id, "expiring entitlement tenant_id")?,
        tenant_name: value.tenant_name,
        workspace_id: parse_optional_uuid(
            &value.workspace_id,
            "expiring entitlement workspace_id",
        )?,
        workspace_name: empty_to_none(value.workspace_name),
        status: value.status,
        effective_to: parse_datetime(&value.effective_to, "expiring entitlement effective_to")?,
    })
}

fn unpublished_change_from_grpc(
    value: billing_pb::AdminUnpublishedEntitlementChange,
) -> Result<UnpublishedEntitlementChange, AppError> {
    Ok(UnpublishedEntitlementChange {
        id: parse_uuid(&value.id, "unpublished entitlement change id")?,
        tenant_id: parse_uuid(&value.tenant_id, "unpublished entitlement change tenant_id")?,
        tenant_name: value.tenant_name,
        event_id: value.event_id,
        created_at: parse_datetime(
            &value.created_at,
            "unpublished entitlement change created_at",
        )?,
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
