use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    grpc_pb::nvbes::developer::v1::{
        DeveloperClientMetadata as GrpcDeveloperClientMetadata, GetClientMetadataRequest,
    },
    http::error::AppError,
};

use super::{developer_client, empty_to_option, grpc_error, request_context};

#[derive(Debug)]
pub struct DeveloperClientMetadata {
    pub marketplace_status: Option<String>,
    pub consent_screen_configured: bool,
    pub health_status: String,
}

pub async fn get_client_metadata(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_ids: Vec<String>,
) -> Result<HashMap<String, DeveloperClientMetadata>, AppError> {
    if client_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut client = developer_client().await?;
    let response = client
        .get_client_metadata(GetClientMetadataRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_ids,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(response.metadata.into_iter().map(client_metadata).collect())
}

fn client_metadata(metadata: GrpcDeveloperClientMetadata) -> (String, DeveloperClientMetadata) {
    (
        metadata.client_id,
        DeveloperClientMetadata {
            marketplace_status: empty_to_option(metadata.marketplace_status),
            consent_screen_configured: metadata.consent_screen_configured,
            health_status: metadata.health_status,
        },
    )
}
