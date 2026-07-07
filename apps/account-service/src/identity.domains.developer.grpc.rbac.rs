use uuid::Uuid;

use crate::{grpc_pb::nvbes::developer::v1::ListDeveloperRolesRequest, http::error::AppError};

use super::{developer_client, grpc_error, request_context};

pub async fn list_developer_roles(
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let mut client = developer_client().await?;
    Ok(client
        .list_developer_roles(ListDeveloperRolesRequest {
            context: Some(request_context(tenant_id, principal_id)),
            tenant_id: tenant_id.to_string(),
            principal_id: principal_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .roles)
}
