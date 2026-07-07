use uuid::Uuid;

use crate::{grpc_pb::nvbes::developer::v1::RecordTokenDebugSessionRequest, http::error::AppError};

use super::{developer_client, grpc_error, request_context};

pub async fn record_token_debug_session(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    token_hash_prefix: String,
    active: bool,
    access_decision: String,
) -> Result<(), AppError> {
    let mut client = developer_client().await?;
    client
        .record_token_debug_session(RecordTokenDebugSessionRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            token_hash_prefix,
            active,
            access_decision,
        })
        .await
        .map_err(grpc_error)?;
    Ok(())
}
