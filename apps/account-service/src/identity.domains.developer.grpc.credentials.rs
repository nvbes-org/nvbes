use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    grpc_pb::nvbes::developer::v1::{
        CountStaleSecretsRequest, DeveloperCredentialSummary as GrpcDeveloperCredentialSummary,
        ListCredentialSummariesRequest, VerifyClientSecretVersionRequest,
    },
    http::error::AppError,
};

use super::{
    developer_client, empty_to_option, grpc_error, parse_optional_time, parse_time, parse_uuid,
    request_context,
};

#[derive(Debug)]
pub struct DeveloperCredentialSummary {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub secret_last4: String,
    pub owner_email: Option<String>,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn list_credential_summaries(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<Vec<DeveloperCredentialSummary>, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_credential_summaries(ListCredentialSummariesRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .credentials
        .into_iter()
        .map(credential_summary)
        .collect()
}

pub async fn count_stale_secrets(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<i64, AppError> {
    let mut client = developer_client().await?;
    Ok(client
        .count_stale_secrets(CountStaleSecretsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .stale_secret_count)
}

pub async fn verify_client_secret_version(
    tenant_id: Uuid,
    client_id: &str,
    client_secret: &str,
) -> Result<bool, AppError> {
    let mut client = developer_client().await?;
    Ok(client
        .verify_client_secret_version(VerifyClientSecretVersionRequest {
            context: Some(system_request_context(tenant_id)),
            tenant_id: tenant_id.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .valid)
}

fn system_request_context(tenant_id: Uuid) -> crate::grpc_pb::nvbes::platform::v1::RequestContext {
    crate::grpc_pb::nvbes::platform::v1::RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: Uuid::nil().to_string(),
        tenant: Some(crate::grpc_pb::nvbes::platform::v1::TenantContext {
            tenant_id: tenant_id.to_string(),
            workspace_id: String::new(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}

fn credential_summary(
    value: GrpcDeveloperCredentialSummary,
) -> Result<DeveloperCredentialSummary, AppError> {
    Ok(DeveloperCredentialSummary {
        id: parse_uuid(&value.id, "id")?,
        client_id: value.client_id,
        name: value.name,
        status: value.status,
        secret_last4: value.secret_last4,
        owner_email: empty_to_option(value.owner_email),
        scopes: value.scopes,
        last_used_at: parse_optional_time(&value.last_used_at, "last_used_at")?,
        created_at: parse_time(&value.created_at, "created_at")?,
        expires_at: parse_optional_time(&value.expires_at, "expires_at")?,
    })
}
