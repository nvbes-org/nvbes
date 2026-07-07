use uuid::Uuid;

use crate::{
    domains::developer::types::{DeveloperLogEntry, DeveloperLogsResponse},
    grpc_pb::nvbes::developer::v1::ListActivityLogsRequest,
    http::error::AppError,
};

use super::{developer_client, empty_to_option, grpc_error, parse_time, request_context};

pub async fn list_activity_logs(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    user_id: Option<Uuid>,
    client_id: Option<String>,
    event_type: Option<String>,
    limit: i64,
) -> Result<DeveloperLogsResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_activity_logs(ListActivityLogsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            user_id: user_id.map(|id| id.to_string()).unwrap_or_default(),
            client_id: client_id.unwrap_or_default(),
            event_type: event_type.unwrap_or_default(),
            limit,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperLogsResponse {
        logs: response
            .logs
            .into_iter()
            .map(|entry| {
                Ok(DeveloperLogEntry {
                    id: entry.id,
                    event_type: entry.event_type,
                    user_id: empty_to_option(entry.user_id),
                    client_id: empty_to_option(entry.client_id),
                    tenant_id: empty_to_option(entry.tenant_id),
                    created_at: parse_time(&entry.created_at, "created_at")?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
    })
}
