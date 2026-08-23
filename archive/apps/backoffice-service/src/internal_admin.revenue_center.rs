use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1 as billing_pb;

#[derive(Debug, Serialize)]
struct RevenueCenterSnapshot {
    captured_payments_30d: Vec<MoneyTotal>,
    open_invoices: Vec<MoneyTotal>,
    overdue_invoices: Vec<MoneyTotal>,
    refunds_30d: Vec<MoneyTotal>,
    disputes_30d: Vec<MoneyTotal>,
    active_subscription_count: i64,
    trialing_subscription_count: i64,
    open_dunning_case_count: i64,
    unresolved_reconciliation_difference_count: i64,
    recent_dunning_cases: Vec<RecentDunningCase>,
    recent_disputes: Vec<RecentDispute>,
    recent_overdue_invoices: Vec<RecentOverdueInvoice>,
    recent_captured_payments: Vec<RecentCapturedPayment>,
}

#[derive(Debug, Serialize)]
struct MoneyTotal {
    currency: String,
    amount_minor: i64,
    object_count: i64,
}

#[derive(Debug, Serialize)]
struct RecentDunningCase {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    status: String,
    policy_state: String,
    opened_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentDispute {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    status: String,
    currency: String,
    amount_minor: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentOverdueInvoice {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    invoice_number: Option<String>,
    status: String,
    currency: String,
    total_minor: i64,
    due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct RecentCapturedPayment {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    status: String,
    currency: String,
    amount_minor: i64,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/revenue-center", get(revenue_center_route))
}

async fn revenue_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RevenueCenterSnapshot>, AppError> {
    let actor_principal_id = actor_principal_id(&headers)?;
    let snapshot = crate::billing_grpc::get_admin_revenue_center(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await?;
    Ok(Json(revenue_center_from_grpc(snapshot)?))
}

fn revenue_center_from_grpc(
    value: billing_pb::AdminRevenueCenterSnapshot,
) -> Result<RevenueCenterSnapshot, AppError> {
    Ok(RevenueCenterSnapshot {
        captured_payments_30d: value
            .captured_payments_30d
            .into_iter()
            .map(money_total_from_grpc)
            .collect(),
        open_invoices: value
            .open_invoices
            .into_iter()
            .map(money_total_from_grpc)
            .collect(),
        overdue_invoices: value
            .overdue_invoices
            .into_iter()
            .map(money_total_from_grpc)
            .collect(),
        refunds_30d: value
            .refunds_30d
            .into_iter()
            .map(money_total_from_grpc)
            .collect(),
        disputes_30d: value
            .disputes_30d
            .into_iter()
            .map(money_total_from_grpc)
            .collect(),
        active_subscription_count: value.active_subscription_count,
        trialing_subscription_count: value.trialing_subscription_count,
        open_dunning_case_count: value.open_dunning_case_count,
        unresolved_reconciliation_difference_count: value
            .unresolved_reconciliation_difference_count,
        recent_dunning_cases: value
            .recent_dunning_cases
            .into_iter()
            .map(recent_dunning_case_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        recent_disputes: value
            .recent_disputes
            .into_iter()
            .map(recent_dispute_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        recent_overdue_invoices: value
            .recent_overdue_invoices
            .into_iter()
            .map(recent_overdue_invoice_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        recent_captured_payments: value
            .recent_captured_payments
            .into_iter()
            .map(recent_captured_payment_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn money_total_from_grpc(value: billing_pb::AdminMoneyTotal) -> MoneyTotal {
    MoneyTotal {
        currency: value.currency,
        amount_minor: value.amount_minor,
        object_count: value.object_count,
    }
}

fn recent_dunning_case_from_grpc(
    value: billing_pb::AdminRecentDunningCase,
) -> Result<RecentDunningCase, AppError> {
    Ok(RecentDunningCase {
        id: parse_uuid(&value.id, "recent dunning case id")?,
        tenant_id: parse_uuid(&value.tenant_id, "recent dunning case tenant_id")?,
        tenant_name: value.tenant_name,
        status: value.status,
        policy_state: value.policy_state,
        opened_at: parse_datetime(&value.opened_at, "recent dunning case opened_at")?,
    })
}

fn recent_dispute_from_grpc(
    value: billing_pb::AdminRecentDispute,
) -> Result<RecentDispute, AppError> {
    Ok(RecentDispute {
        id: parse_uuid(&value.id, "recent dispute id")?,
        tenant_id: parse_uuid(&value.tenant_id, "recent dispute tenant_id")?,
        tenant_name: value.tenant_name,
        status: value.status,
        currency: value.currency,
        amount_minor: value.amount_minor,
        created_at: parse_datetime(&value.created_at, "recent dispute created_at")?,
    })
}

fn recent_overdue_invoice_from_grpc(
    value: billing_pb::AdminRecentOverdueInvoice,
) -> Result<RecentOverdueInvoice, AppError> {
    Ok(RecentOverdueInvoice {
        id: parse_uuid(&value.id, "recent overdue invoice id")?,
        tenant_id: parse_uuid(&value.tenant_id, "recent overdue invoice tenant_id")?,
        tenant_name: value.tenant_name,
        invoice_number: empty_to_none(value.invoice_number),
        status: value.status,
        currency: value.currency,
        total_minor: value.total_minor,
        due_at: parse_optional_datetime(&value.due_at, "recent overdue invoice due_at")?,
    })
}

fn recent_captured_payment_from_grpc(
    value: billing_pb::AdminRecentCapturedPayment,
) -> Result<RecentCapturedPayment, AppError> {
    Ok(RecentCapturedPayment {
        id: parse_uuid(&value.id, "recent captured payment id")?,
        tenant_id: parse_uuid(&value.tenant_id, "recent captured payment tenant_id")?,
        tenant_name: value.tenant_name,
        status: value.status,
        currency: value.currency,
        amount_minor: value.amount_minor,
        created_at: parse_datetime(&value.created_at, "recent captured payment created_at")?,
    })
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_optional_datetime(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_datetime(value, field).map(Some)
    }
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
