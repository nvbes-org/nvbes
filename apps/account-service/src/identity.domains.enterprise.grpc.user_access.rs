use uuid::Uuid;

use crate::{
    domains::authz::AdminScope,
    grpc_pb::nvbes::enterprise::v1::{
        ReactivateUserAccessRequest, SuspendUserAccessRequest, UpdateUserAccessRequest,
    },
    http::error::AppError,
};

pub async fn update_user_access(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    target_principal_id: Uuid,
    scope: AdminScope,
    role: &str,
    workspace_ids: Vec<Uuid>,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .update_user_access(UpdateUserAccessRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            target_principal_id: target_principal_id.to_string(),
            role: role.to_string(),
            workspace_ids: workspace_ids.into_iter().map(|id| id.to_string()).collect(),
            scope: scope_label(scope).to_string(),
            organization_id: organization_id(scope),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn suspend_user_access(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    target_principal_id: Uuid,
    scope: AdminScope,
    reason: String,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .suspend_user_access(SuspendUserAccessRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            target_principal_id: target_principal_id.to_string(),
            reason,
            scope: scope_label(scope).to_string(),
            organization_id: organization_id(scope),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn reactivate_user_access(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    target_principal_id: Uuid,
    scope: AdminScope,
    reason: String,
    workspace_ids: Vec<Uuid>,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .reactivate_user_access(ReactivateUserAccessRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            target_principal_id: target_principal_id.to_string(),
            reason,
            workspace_ids: workspace_ids.into_iter().map(|id| id.to_string()).collect(),
            scope: scope_label(scope).to_string(),
            organization_id: organization_id(scope),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

fn scope_label(scope: AdminScope) -> &'static str {
    match scope {
        AdminScope::Tenant => "tenant",
        AdminScope::Organization(_) => "organization",
    }
}

fn organization_id(scope: AdminScope) -> String {
    match scope {
        AdminScope::Tenant => String::new(),
        AdminScope::Organization(organization_id) => organization_id.to_string(),
    }
}
