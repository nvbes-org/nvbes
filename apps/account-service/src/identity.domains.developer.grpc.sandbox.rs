use uuid::Uuid;

use crate::{
    domains::developer::types::{
        DeveloperSandboxResponse, DeveloperSandboxTenantSummary, UpsertDeveloperSandboxInput,
    },
    grpc_pb::nvbes::developer::v1::{CreateSandboxRequest, GetSandboxRequest, ResetSandboxRequest},
    http::error::AppError,
};

use super::{
    developer_client, grpc_error, parse_optional_time, parse_time, parse_uuid, request_context,
};

pub async fn get_sandbox(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperSandboxResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .get_sandbox(GetSandboxRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperSandboxResponse {
        sandbox: response.sandbox.map(sandbox_summary).transpose()?,
    })
}

pub async fn upsert_sandbox(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: UpsertDeveloperSandboxInput,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    let mut client = developer_client().await?;
    let sandbox = client
        .create_sandbox(CreateSandboxRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            template: input.data_profile.unwrap_or_default(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    sandbox_summary(sandbox)
}

pub async fn reset_sandbox(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    let mut client = developer_client().await?;
    let sandbox = client
        .reset_sandbox(ResetSandboxRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            sandbox_id: String::new(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    sandbox_summary(sandbox)
}

fn sandbox_summary(
    sandbox: crate::grpc_pb::nvbes::developer::v1::Sandbox,
) -> Result<DeveloperSandboxTenantSummary, AppError> {
    Ok(DeveloperSandboxTenantSummary {
        tenant_id: parse_uuid(&sandbox.tenant_id, "tenant_id")?,
        sandbox_tenant_id: parse_uuid(&sandbox.sandbox_id, "sandbox_id")?,
        sandbox_name: sandbox.sandbox_name,
        sandbox_slug: sandbox.sandbox_slug,
        status: sandbox.status,
        data_profile: sandbox.data_profile,
        reset_requested_at: parse_optional_time(&sandbox.reset_at, "reset_at")?,
        updated_at: parse_time(&sandbox.updated_at, "updated_at")?,
    })
}
