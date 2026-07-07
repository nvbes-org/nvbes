use std::collections::BTreeMap;

use uuid::Uuid;

use crate::{
    domains::{authz::AdminScope, enterprise::types::EnterpriseAuditEvent},
    grpc_pb::nvbes::enterprise::v1::{
        AuditEvent as GrpcAuditEvent, ListAuditEventsRequest, RecordDeveloperSecretRevokedRequest,
    },
    http::error::AppError,
};

pub async fn record_developer_secret_revoked(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    version_id: Uuid,
    client_id: &str,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .record_developer_secret_revoked(RecordDeveloperSecretRevokedRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            version_id: version_id.to_string(),
            client_id: client_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn list_audit_events(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    scope: AdminScope,
    limit: i32,
) -> Result<Vec<EnterpriseAuditEvent>, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .list_audit_events(ListAuditEventsRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            scope: scope_label(scope).to_string(),
            organization_id: organization_id(scope),
            limit,
        })
        .await
        .map_err(super::grpc_error)?
        .into_inner();
    response
        .events
        .into_iter()
        .map(audit_event_from_grpc)
        .collect()
}

fn audit_event_from_grpc(value: GrpcAuditEvent) -> Result<EnterpriseAuditEvent, AppError> {
    Ok(EnterpriseAuditEvent {
        id: parse_uuid(&value.event_id, "event_id")?,
        event_type: value.event_type,
        actor_id: optional_uuid(&value.actor_id, "actor_id")?,
        actor_email: optional_text(value.actor_email),
        target_type: optional_text(value.target_type),
        target_id: optional_uuid(&value.target_id, "target_id")?,
        metadata: metadata(&value.metadata_json)?,
        created_at: parse_time(&value.created_at, "created_at")?,
    })
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

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_audit_event",
            format!("Enterprise gRPC returned invalid {field}: {error}"),
        )
    })
}

fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_time(value: &str, field: &'static str) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    chrono::DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_audit_event",
                format!("Enterprise gRPC returned invalid {field}: {error}"),
            )
        })
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn metadata(value: &str) -> Result<Option<BTreeMap<String, serde_json::Value>>, AppError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(value).map(Some).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_audit_event",
            format!("Enterprise gRPC returned invalid metadata_json: {error}"),
        )
    })
}
