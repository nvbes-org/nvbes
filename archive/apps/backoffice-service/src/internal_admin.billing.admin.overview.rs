use axum::{Json, Router, extract::Path, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct BillingOverview {
    open_invoice_count: i64,
    overdue_invoice_count: i64,
    open_invoice_total_minor: i64,
    failed_provider_event_count: i64,
    pending_refund_count: i64,
    active_subscription_count: i64,
    captured_payment_total_minor_30d: i64,
    last_billing_audit_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/billing/admin/overview",
        get(overview_route),
    )
}

async fn overview_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingOverview>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let overview = crate::billing_grpc::get_admin_billing_overview(
        &state.billing_grpc_endpoint,
        access,
        workspace_id,
    )
    .await?;
    Ok(Json(billing_overview_from_grpc(overview)?))
}

fn billing_overview_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::AdminBillingOverview,
) -> Result<BillingOverview, AppError> {
    Ok(BillingOverview {
        open_invoice_count: value.open_invoice_count,
        overdue_invoice_count: value.overdue_invoice_count,
        open_invoice_total_minor: value.open_invoice_total_minor,
        failed_provider_event_count: value.failed_provider_event_count,
        pending_refund_count: value.pending_refund_count,
        active_subscription_count: value.active_subscription_count,
        captured_payment_total_minor_30d: value.captured_payment_total_minor_30d,
        last_billing_audit_at: parse_optional_datetime(&value.last_billing_audit_at)?,
    })
}

fn parse_optional_datetime(value: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        DateTime::parse_from_rfc3339(value)
            .map(|value| Some(value.with_timezone(&Utc)))
            .map_err(|_| AppError::internal("billing_grpc_decode", "last_billing_audit_at"))
    }
}
