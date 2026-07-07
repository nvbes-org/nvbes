use axum::http::StatusCode;
use chrono::{DateTime, NaiveDate, Utc};
use tonic::{Code, transport::Channel};
use uuid::Uuid;

use crate::{
    error::AppError,
    grpc_pb::nvbes::{
        billing::v1::{
            AdminCommandCenterBillingMetrics, AdminOperationsCenterSnapshot,
            GetAdminCommandCenterBillingMetricsRequest, GetAdminOperationsCenterRequest,
            billing_service_client::BillingServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
};

const BILLING_GRPC_ENDPOINT_ENV: &str = "NVBES_BILLING_GRPC_ENDPOINT";

pub struct BackofficeBillingCommandMetrics {
    pub pending_kyc_approval_count: i64,
    pub provider_failure_count: i64,
    pub overdue_invoice_count: i64,
    pub failed_payment_count: i64,
}

pub struct BackofficeBillingOperationsSnapshot {
    pub provider_event_failure_count: i64,
    pub provider_event_backlog_count: i64,
    pub export_pending_count: i64,
    pub export_failed_count: i64,
    pub reconciliation_pending_count: i64,
    pub reconciliation_failed_count: i64,
    pub unresolved_reconciliation_difference_count: i64,
    pub recent_provider_failures: Vec<BackofficeRecentProviderFailure>,
    pub recent_export_runs: Vec<BackofficeRecentExportRun>,
    pub recent_reconciliation_differences: Vec<BackofficeRecentReconciliationDifference>,
}

pub struct BackofficeRecentProviderFailure {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub provider: String,
    pub provider_event_id: String,
    pub event_type: String,
    pub status: String,
    pub received_at: DateTime<Utc>,
}

pub struct BackofficeRecentExportRun {
    pub id: Uuid,
    pub export_type: String,
    pub status: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct BackofficeRecentReconciliationDifference {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub difference_type: String,
    pub severity: String,
    pub created_at: DateTime<Utc>,
}

pub fn billing_grpc_endpoint(default_api_port: u16) -> String {
    std::env::var(BILLING_GRPC_ENDPOINT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            default_api_port.checked_add(21).map_or_else(
                || "http://127.0.0.1:4021".to_string(),
                |port| format!("http://127.0.0.1:{port}"),
            )
        })
}

pub async fn get_admin_command_center_billing_metrics(
    endpoint: &str,
    actor_id: Uuid,
) -> Result<BackofficeBillingCommandMetrics, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_command_center_billing_metrics(GetAdminCommandCenterBillingMetricsRequest {
            context: Some(request_context(actor_id)),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    Ok(command_metrics(response))
}

pub async fn get_admin_operations_center(
    endpoint: &str,
    actor_id: Uuid,
) -> Result<BackofficeBillingOperationsSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_operations_center(GetAdminOperationsCenterRequest {
            context: Some(request_context(actor_id)),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    operations_snapshot(response)
}

pub(crate) async fn billing_client(
    endpoint: &str,
) -> Result<BillingServiceClient<Channel>, AppError> {
    BillingServiceClient::connect(endpoint.to_string())
        .await
        .map_err(|error| AppError::internal("billing_grpc_connect_failed", error.to_string()))
}

fn request_context(actor_id: Uuid) -> RequestContext {
    let request_id = Uuid::new_v4().to_string();
    RequestContext {
        request_id: request_id.clone(),
        correlation_id: request_id,
        actor_principal_id: actor_id.to_string(),
        tenant: None,
    }
}

pub(crate) fn request_context_for_workspace(
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
) -> RequestContext {
    let mut context = request_context(actor_id);
    context.tenant = Some(TenantContext {
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.to_string(),
        region_id: String::new(),
        data_residency: String::new(),
    });
    context
}

fn command_metrics(value: AdminCommandCenterBillingMetrics) -> BackofficeBillingCommandMetrics {
    BackofficeBillingCommandMetrics {
        pending_kyc_approval_count: value.pending_kyc_approval_count,
        provider_failure_count: value.provider_failure_count,
        overdue_invoice_count: value.overdue_invoice_count,
        failed_payment_count: value.failed_payment_count,
    }
}

fn operations_snapshot(
    value: AdminOperationsCenterSnapshot,
) -> Result<BackofficeBillingOperationsSnapshot, AppError> {
    Ok(BackofficeBillingOperationsSnapshot {
        provider_event_failure_count: value.provider_event_failure_count,
        provider_event_backlog_count: value.provider_event_backlog_count,
        export_pending_count: value.export_pending_count,
        export_failed_count: value.export_failed_count,
        reconciliation_pending_count: value.reconciliation_pending_count,
        reconciliation_failed_count: value.reconciliation_failed_count,
        unresolved_reconciliation_difference_count: value
            .unresolved_reconciliation_difference_count,
        recent_provider_failures: value
            .recent_provider_failures
            .into_iter()
            .map(|item| {
                Ok(BackofficeRecentProviderFailure {
                    id: parse_uuid(&item.id)?,
                    tenant_id: parse_optional_uuid(&item.tenant_id)?,
                    tenant_name: empty_to_none(item.tenant_name),
                    provider: item.provider,
                    provider_event_id: item.provider_event_id,
                    event_type: item.event_type,
                    status: item.status,
                    received_at: parse_datetime(&item.received_at)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
        recent_export_runs: value
            .recent_export_runs
            .into_iter()
            .map(|item| {
                Ok(BackofficeRecentExportRun {
                    id: parse_uuid(&item.id)?,
                    export_type: item.export_type,
                    status: item.status,
                    period_start: parse_optional_date(&item.period_start)?,
                    period_end: parse_optional_date(&item.period_end)?,
                    created_at: parse_datetime(&item.created_at)?,
                    updated_at: parse_datetime(&item.updated_at)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
        recent_reconciliation_differences: value
            .recent_reconciliation_differences
            .into_iter()
            .map(|item| {
                Ok(BackofficeRecentReconciliationDifference {
                    id: parse_uuid(&item.id)?,
                    tenant_id: parse_optional_uuid(&item.tenant_id)?,
                    tenant_name: empty_to_none(item.tenant_name),
                    difference_type: item.difference_type,
                    severity: item.severity,
                    created_at: parse_datetime(&item.created_at)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
    })
}

pub(crate) fn grpc_error(error: tonic::Status) -> AppError {
    match error.code() {
        Code::InvalidArgument => AppError::bad_request("billing_grpc_error", error.message()),
        Code::Unauthenticated => AppError::unauthorized("billing_grpc_error", error.message()),
        Code::PermissionDenied => AppError::forbidden("billing_grpc_error", error.message()),
        Code::NotFound => AppError::not_found("billing_grpc_error", error.message()),
        Code::FailedPrecondition => AppError::conflict("billing_grpc_error", error.message()),
        Code::Unavailable | Code::DeadlineExceeded => AppError::new(
            StatusCode::BAD_GATEWAY,
            "billing_grpc_unavailable",
            error.message(),
        ),
        _ => AppError::internal("billing_grpc_error", error.message()),
    }
}

pub(crate) fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value)
        .map_err(|error| AppError::internal("billing_grpc_invalid_uuid", error.to_string()))
}

fn parse_optional_uuid(value: &str) -> Result<Option<Uuid>, AppError> {
    empty_to_none(value.to_string()).map_or(Ok(None), |value| parse_uuid(&value).map(Some))
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| AppError::internal("billing_grpc_invalid_datetime", error.to_string()))
}

fn parse_optional_date(value: &str) -> Result<Option<NaiveDate>, AppError> {
    empty_to_none(value.to_string()).map_or(Ok(None), |value| {
        NaiveDate::parse_from_str(&value, "%Y-%m-%d")
            .map(Some)
            .map_err(|error| AppError::internal("billing_grpc_invalid_date", error.to_string()))
    })
}

pub(crate) fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}
