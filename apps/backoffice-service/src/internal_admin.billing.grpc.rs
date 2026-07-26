use std::time::Duration;

use axum::http::StatusCode;
use chrono::{DateTime, NaiveDate, Utc};
use tonic::{Request, transport::Channel};
use uuid::Uuid;

use crate::{
    billing_admin_types::BackofficeAccess,
    error::AppError,
    grpc_pb::nvbes::{
        billing::v1::{
            AdminEntitlementsActionRequest, AdminRiskActionKind, AdminRiskActionRequest,
            AdminUsageActionRequest, BuildAdminFinanceExportRequest,
            GetAdminBillingOverviewRequest, GetAdminBillingPlatformCenterRequest,
            GetAdminCommandCenterBillingMetricsRequest, GetAdminEntitlementsCenterRequest,
            GetAdminOperationsCenterRequest, GetAdminRevenueCenterRequest,
            GetAdminRiskDecisionCenterRequest, GetAdminTenantBillingSummaryRequest,
            GetAdminUsageCenterRequest, GetAdminWorkspaceBillingSummaryRequest,
            ListAdminProviderEventFailuresRequest, SearchAdminBillingRequest,
            SimulateAdminBillingRoutingRequest, billing_service_client::BillingServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
};

const BILLING_GRPC_ENDPOINT_ENV: &str = "NVBES_BILLING_GRPC_ENDPOINT";
const BILLING_GRPC_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct BackofficeBillingOperationsSnapshot {
    pub(crate) provider_event_failure_count: i64,
    pub(crate) provider_event_backlog_count: i64,
    pub(crate) export_pending_count: i64,
    pub(crate) export_failed_count: i64,
    pub(crate) reconciliation_pending_count: i64,
    pub(crate) reconciliation_failed_count: i64,
    pub(crate) unresolved_reconciliation_difference_count: i64,
    pub(crate) recent_provider_failures: Vec<BackofficeRecentProviderFailure>,
    pub(crate) recent_export_runs: Vec<BackofficeRecentExportRun>,
    pub(crate) recent_reconciliation_differences: Vec<BackofficeRecentReconciliationDifference>,
}

pub(crate) struct BackofficeRecentProviderFailure {
    pub(crate) id: Uuid,
    pub(crate) tenant_id: Option<Uuid>,
    pub(crate) tenant_name: Option<String>,
    pub(crate) provider: String,
    pub(crate) provider_event_id: String,
    pub(crate) event_type: String,
    pub(crate) status: String,
    pub(crate) received_at: DateTime<Utc>,
}

pub(crate) struct BackofficeRecentExportRun {
    pub(crate) id: Uuid,
    pub(crate) export_type: String,
    pub(crate) status: String,
    pub(crate) period_start: Option<NaiveDate>,
    pub(crate) period_end: Option<NaiveDate>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

pub(crate) struct BackofficeRecentReconciliationDifference {
    pub(crate) id: Uuid,
    pub(crate) tenant_id: Option<Uuid>,
    pub(crate) tenant_name: Option<String>,
    pub(crate) difference_type: String,
    pub(crate) severity: String,
    pub(crate) created_at: DateTime<Utc>,
}

pub(crate) fn billing_grpc_endpoint(default_api_port: u16) -> anyhow::Result<String> {
    match std::env::var(BILLING_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => Err(anyhow::anyhow!(
            "{BILLING_GRPC_ENDPOINT_ENV} must not be empty"
        )),
        Err(std::env::VarError::NotPresent) => {
            let port = default_api_port
                .checked_add(21)
                .ok_or_else(|| anyhow::anyhow!("Default Billing gRPC port overflowed"))?;
            Ok(format!("http://127.0.0.1:{port}"))
        }
        Err(error) => Err(anyhow::anyhow!(
            "{BILLING_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

pub(crate) async fn billing_client(
    endpoint: &str,
) -> Result<BillingServiceClient<Channel>, AppError> {
    BillingServiceClient::connect(endpoint.to_string())
        .await
        .map_err(|error| AppError::internal("billing_grpc_unavailable", error.to_string()))
}

pub(crate) fn request_context(
    access: BackofficeAccess,
    workspace_id: Option<Uuid>,
) -> RequestContext {
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: access.actor_principal_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: access.tenant_id.to_string(),
            workspace_id: workspace_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            region_id: "backoffice-service".to_string(),
            data_residency: "internal".to_string(),
        }),
    }
}

pub(crate) fn grpc_request<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    request.set_timeout(BILLING_GRPC_TIMEOUT);
    request
}

pub(crate) fn grpc_status(error: tonic::Status) -> AppError {
    match error.code() {
        tonic::Code::InvalidArgument => {
            AppError::bad_request("billing_grpc_invalid_argument", error.message())
        }
        tonic::Code::FailedPrecondition | tonic::Code::AlreadyExists => {
            AppError::conflict("billing_grpc_conflict", error.message())
        }
        tonic::Code::NotFound => AppError::not_found("billing_grpc_not_found", error.message()),
        tonic::Code::PermissionDenied => {
            AppError::forbidden("billing_grpc_permission_denied", error.message())
        }
        _ => AppError::internal("billing_grpc_error", error.message()),
    }
}

pub(crate) fn grpc_error(error: tonic::Status) -> AppError {
    match error.code() {
        tonic::Code::Unavailable | tonic::Code::DeadlineExceeded => AppError::new(
            StatusCode::BAD_GATEWAY,
            "billing_grpc_unavailable",
            error.message(),
        ),
        _ => grpc_status(error),
    }
}

pub(crate) fn request_context_for_workspace(
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
) -> RequestContext {
    request_context(
        BackofficeAccess {
            tenant_id,
            actor_principal_id: actor_id,
        },
        Some(workspace_id),
    )
}

pub(crate) fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value)
        .map_err(|error| AppError::internal("billing_grpc_invalid_uuid", error.to_string()))
}

pub(crate) fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

pub(crate) async fn get_admin_command_center_billing_metrics(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminCommandCenterBillingMetrics, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_command_center_billing_metrics(grpc_request(
            GetAdminCommandCenterBillingMetricsRequest {
                context: Some(request_context(access, None)),
            },
        ))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_operations_center(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<BackofficeBillingOperationsSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_operations_center(grpc_request(GetAdminOperationsCenterRequest {
            context: Some(request_context(access, None)),
        }))
        .await
        .map_err(grpc_status)?;
    operations_snapshot(response.into_inner())
}

fn operations_snapshot(
    value: crate::grpc_pb::nvbes::billing::v1::AdminOperationsCenterSnapshot,
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

pub(crate) async fn get_admin_billing_overview(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminBillingOverview, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_billing_overview(grpc_request(GetAdminBillingOverviewRequest {
            context: Some(request_context(access, Some(workspace_id))),
            workspace_id: workspace_id.to_string(),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_tenant_billing_summary(
    endpoint: &str,
    access: BackofficeAccess,
    tenant_id: Uuid,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminTenantBillingSummary, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_tenant_billing_summary(grpc_request(GetAdminTenantBillingSummaryRequest {
            context: Some(request_context(access, None)),
            tenant_id: tenant_id.to_string(),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_workspace_billing_summary(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminWorkspaceBillingSummary, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_workspace_billing_summary(grpc_request(GetAdminWorkspaceBillingSummaryRequest {
            context: Some(request_context(access, Some(workspace_id))),
            workspace_id: workspace_id.to_string(),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn list_admin_provider_event_failures(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    limit: i32,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminProviderEventFailures, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .list_admin_provider_event_failures(grpc_request(ListAdminProviderEventFailuresRequest {
            context: Some(request_context(access, Some(workspace_id))),
            workspace_id: workspace_id.to_string(),
            limit,
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn search_admin_billing(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    query: String,
    limit: i32,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminBillingSearchResults, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .search_admin_billing(grpc_request(SearchAdminBillingRequest {
            context: Some(request_context(access, Some(workspace_id))),
            workspace_id: workspace_id.to_string(),
            query,
            limit,
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn build_admin_finance_export(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    export_type: String,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminFinanceExport, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .build_admin_finance_export(grpc_request(BuildAdminFinanceExportRequest {
            context: Some(request_context(access, Some(workspace_id))),
            workspace_id: workspace_id.to_string(),
            export_type,
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_billing_platform_center(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminBillingPlatformCenterSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_billing_platform_center(grpc_request(GetAdminBillingPlatformCenterRequest {
            context: Some(request_context(access, None)),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_usage_center(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminUsageCenterSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_usage_center(grpc_request(GetAdminUsageCenterRequest {
            context: Some(request_context(access, None)),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn run_usage_grpc_action(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: AdminUsageActionRequest,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminUsageActionResult, AppError> {
    let mut client = billing_client(endpoint).await?;
    let mut request = request;
    request.context = Some(request_context(access, Some(workspace_id)));
    request.workspace_id = workspace_id.to_string();
    let response = client
        .run_admin_usage_action(grpc_request(request))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_entitlements_center(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminEntitlementsCenterSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_entitlements_center(grpc_request(GetAdminEntitlementsCenterRequest {
            context: Some(request_context(access, None)),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn run_entitlements_grpc_action(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: AdminEntitlementsActionRequest,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminEntitlementsActionResult, AppError> {
    let mut client = billing_client(endpoint).await?;
    let mut request = request;
    request.context = Some(request_context(access, Some(workspace_id)));
    request.workspace_id = workspace_id.to_string();
    let response = client
        .run_admin_entitlements_action(grpc_request(request))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn simulate_admin_billing_routing(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: SimulateAdminBillingRoutingRequest,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminBillingRoutingSimulationResult, AppError> {
    let mut client = billing_client(endpoint).await?;
    let mut request = request;
    request.context = Some(request_context(access, Some(workspace_id)));
    request.workspace_id = workspace_id.to_string();
    let response = client
        .simulate_admin_billing_routing(grpc_request(request))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_revenue_center(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminRevenueCenterSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_revenue_center(grpc_request(GetAdminRevenueCenterRequest {
            context: Some(request_context(access, None)),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn get_admin_risk_decision_center(
    endpoint: &str,
    access: BackofficeAccess,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminRiskDecisionCenterSnapshot, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .get_admin_risk_decision_center(grpc_request(GetAdminRiskDecisionCenterRequest {
            context: Some(request_context(access, None)),
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

pub(crate) async fn run_risk_grpc_action(
    endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    action_kind: AdminRiskActionKind,
    target_id: Uuid,
    reason: String,
) -> Result<crate::grpc_pb::nvbes::billing::v1::AdminRiskActionResult, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .run_admin_risk_action(grpc_request(AdminRiskActionRequest {
            context: Some(request_context(access, Some(workspace_id))),
            workspace_id: workspace_id.to_string(),
            action_kind: action_kind as i32,
            target_id: target_id.to_string(),
            reason,
        }))
        .await
        .map_err(grpc_status)?;
    Ok(response.into_inner())
}

#[cfg(test)]
mod tests {
    use super::billing_grpc_endpoint;

    #[test]
    fn billing_grpc_endpoint_defaults_to_billing_grpc_offset() {
        assert_eq!(
            billing_grpc_endpoint(3000).unwrap(),
            "http://127.0.0.1:3021"
        );
    }
}
