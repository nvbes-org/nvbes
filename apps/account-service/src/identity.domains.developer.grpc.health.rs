use uuid::Uuid;

use crate::{
    domains::developer::types::{
        DeveloperHealthCheckSummary, DeveloperHealthChecksResponse,
        RunDeveloperHealthChecksResponse,
    },
    grpc_pb::nvbes::developer::v1::{ListHealthChecksRequest, RunHealthChecksRequest},
    http::error::AppError,
};

use super::{developer_client, grpc_error, parse_time, parse_uuid, request_context};

pub async fn list_health_checks(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperHealthChecksResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_health_checks(ListHealthChecksRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperHealthChecksResponse {
        checks: response
            .checks
            .into_iter()
            .map(health_check_summary)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn run_health_checks(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<RunDeveloperHealthChecksResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .run_health_checks(RunHealthChecksRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(RunDeveloperHealthChecksResponse {
        checks: response
            .checks
            .into_iter()
            .map(health_check_summary)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn health_check_summary(
    check: crate::grpc_pb::nvbes::developer::v1::HealthCheck,
) -> Result<DeveloperHealthCheckSummary, AppError> {
    Ok(DeveloperHealthCheckSummary {
        id: parse_uuid(&check.id, "id")?,
        target_type: check.target_type,
        target_id: check.target_id,
        check_kind: check.check_kind,
        status: check.status,
        summary: check.summary,
        checked_at: parse_time(&check.checked_at, "checked_at")?,
    })
}
