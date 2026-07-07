use uuid::Uuid;

use crate::{
    domains::developer::types::DeveloperOverviewResponse,
    grpc_pb::nvbes::developer::v1::GetOverviewSummaryRequest, http::error::AppError,
};

use super::{developer_client, grpc_error, parse_uuid, request_context};

pub async fn get_overview_summary(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperOverviewResponse, AppError> {
    let mut client = developer_client().await?;
    let summary = client
        .get_overview_summary(GetOverviewSummaryRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperOverviewResponse {
        tenant_id: parse_uuid(&summary.tenant_id, "tenant_id")?,
        oauth_clients: summary.oauth_clients,
        marketplace_pending: summary.marketplace_pending,
        high_risk_scopes: summary.high_risk_scopes,
        failed_webhook_deliveries: summary.failed_webhook_deliveries,
        unhealthy_integrations: summary.unhealthy_integrations,
        active_sandboxes: summary.active_sandboxes,
    })
}
