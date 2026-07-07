use uuid::Uuid;

use crate::{grpc_pb::nvbes::cloud::v1::ListWorkspacesRequest, http::error::AppError};

use super::{CloudWorkspaceSummary, cloud_client, grpc_error, request_context, workspace_summary};

pub async fn list_tenant_workspaces(
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
    actor_principal_id: Uuid,
) -> Result<Vec<CloudWorkspaceSummary>, AppError> {
    let mut client = cloud_client().await?;
    let response = client
        .list_workspaces(ListWorkspacesRequest {
            context: Some(request_context(
                Some(tenant_id),
                Uuid::nil(),
                actor_principal_id,
                "",
            )),
            page: None,
            tenant_id: tenant_id.to_string(),
            organization_id: organization_id.map(|id| id.to_string()).unwrap_or_default(),
            include_tenant_scope: true,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .workspaces
        .into_iter()
        .map(workspace_summary)
        .collect()
}
