use axum::http::StatusCode;
use tonic::Code;
use uuid::Uuid;

use crate::{
    domains::developer::types::{
        CreateScopeInput, DeveloperScopeRegistryEntry, DeveloperScopeRegistryResponse,
        UpdateScopeInput,
    },
    grpc_pb::nvbes::developer::v1::{
        CreateScopeRequest, DeleteScopeRequest, ListScopesRequest, UpdateScopeRequest,
    },
    http::error::AppError,
};

use super::{developer_client, global_request_context, grpc_error, request_context};

pub async fn list_scopes(
    actor_principal_id: Uuid,
) -> Result<DeveloperScopeRegistryResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_scopes(ListScopesRequest {
            context: Some(global_request_context(actor_principal_id)),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperScopeRegistryResponse {
        scopes: response
            .scopes
            .into_iter()
            .map(scope_entry)
            .collect::<Vec<_>>(),
    })
}

pub async fn create_scope(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: CreateScopeInput,
) -> Result<DeveloperScopeRegistryEntry, AppError> {
    let mut client = developer_client().await?;
    let scope = client
        .create_scope(CreateScopeRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            scope_key: input.scope_key,
            display_name: input.display_name,
            description: input.description,
            risk: input.risk,
            owner_team: input.owner_team,
            lifecycle: input.lifecycle.unwrap_or_default(),
            allowed_audiences: input.allowed_audiences,
        })
        .await
        .map_err(scope_grpc_error)?
        .into_inner();

    Ok(scope_entry(scope))
}

pub async fn update_scope(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    scope_key: String,
    input: UpdateScopeInput,
) -> Result<DeveloperScopeRegistryEntry, AppError> {
    let mut client = developer_client().await?;
    let scope = client
        .update_scope(UpdateScopeRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            scope_key,
            display_name: input.display_name,
            description: input.description,
            risk: input.risk,
            owner_team: input.owner_team,
            lifecycle: input.lifecycle,
            allowed_audiences: input.allowed_audiences,
        })
        .await
        .map_err(scope_grpc_error)?
        .into_inner();

    Ok(scope_entry(scope))
}

pub async fn delete_scope(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    scope_key: String,
) -> Result<(), AppError> {
    let mut client = developer_client().await?;
    client
        .delete_scope(DeleteScopeRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            scope_key,
        })
        .await
        .map_err(scope_grpc_error)?;
    Ok(())
}

fn scope_entry(
    scope: crate::grpc_pb::nvbes::developer::v1::DeveloperScope,
) -> DeveloperScopeRegistryEntry {
    DeveloperScopeRegistryEntry {
        scope_key: scope.scope_key,
        display_name: scope.display_name,
        description: scope.description,
        risk: scope.risk,
        owner_team: scope.owner_team,
        lifecycle: scope.lifecycle,
        allowed_audiences: scope.allowed_audiences,
    }
}

fn scope_grpc_error(error: tonic::Status) -> AppError {
    if error.code() == Code::NotFound {
        return AppError::new(
            StatusCode::NOT_FOUND,
            "scope_not_found",
            "The requested scope was not found.",
        );
    }
    grpc_error(error)
}
