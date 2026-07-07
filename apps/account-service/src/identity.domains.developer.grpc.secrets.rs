use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domains::developer::types::DeveloperSecretVersionSummary,
    grpc_pb::nvbes::developer::v1::{
        ListSecretVersionsRequest, RecordSecretRotationRequest, RevokeSecretVersionRequest,
    },
    http::error::AppError,
};

use super::{
    developer_client, grpc_error, parse_optional_time, parse_time, parse_uuid, request_context,
};

pub struct DeveloperSecretRotationRecord {
    pub client_id: String,
    pub active_version_id: Uuid,
    pub previous_version_id: Uuid,
    pub overlap_ends_at: DateTime<Utc>,
    pub rotated_at: DateTime<Utc>,
}

pub async fn list_secret_versions(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_id: String,
) -> Result<Vec<DeveloperSecretVersionSummary>, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_secret_versions(ListSecretVersionsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_id,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .secret_versions
        .into_iter()
        .map(secret_version_summary)
        .collect()
}

pub async fn record_secret_rotation(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_id: String,
    current_secret_hash: String,
    new_secret_hash: String,
    new_secret_last4: String,
    overlap_ends_at: DateTime<Utc>,
    rotated_at: DateTime<Utc>,
) -> Result<DeveloperSecretRotationRecord, AppError> {
    let mut client = developer_client().await?;
    let rotation = client
        .record_secret_rotation(RecordSecretRotationRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_id,
            current_secret_hash,
            new_secret_hash,
            new_secret_last4,
            overlap_ends_at: overlap_ends_at.to_rfc3339(),
            rotated_at: rotated_at.to_rfc3339(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperSecretRotationRecord {
        client_id: rotation.client_id,
        active_version_id: parse_uuid(&rotation.active_version_id, "active_version_id")?,
        previous_version_id: parse_uuid(&rotation.previous_version_id, "previous_version_id")?,
        overlap_ends_at: parse_time(&rotation.overlap_ends_at, "overlap_ends_at")?,
        rotated_at: parse_time(&rotation.rotated_at, "rotated_at")?,
    })
}

pub async fn revoke_secret_version(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    version_id: Uuid,
) -> Result<String, AppError> {
    let mut client = developer_client().await?;
    let revocation = client
        .revoke_secret_version(RevokeSecretVersionRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            version_id: version_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(revocation.client_id)
}

fn secret_version_summary(
    version: crate::grpc_pb::nvbes::developer::v1::SecretVersion,
) -> Result<DeveloperSecretVersionSummary, AppError> {
    Ok(DeveloperSecretVersionSummary {
        id: parse_uuid(&version.id, "id")?,
        client_id: version.client_id,
        status: version.status,
        secret_last4: version.secret_last4,
        created_at: parse_time(&version.created_at, "created_at")?,
        expires_at: parse_optional_time(&version.expires_at, "expires_at")?,
        revoked_at: parse_optional_time(&version.revoked_at, "revoked_at")?,
    })
}
