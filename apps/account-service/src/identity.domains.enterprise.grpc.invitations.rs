use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use tonic::Code;
use uuid::Uuid;

use crate::{
    domains::{
        authz::AdminScope,
        enterprise::{
            policy,
            types::{EnterpriseInvitation, EnterpriseInvitationsResponse},
        },
    },
    grpc_pb::nvbes::enterprise::v1::{CreateInvitationsRequest, Invitation as GrpcInvitation},
    http::error::AppError,
};

pub async fn create_invitations(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    scope: AdminScope,
    emails: Vec<String>,
    role: &str,
    workspace_ids: Vec<Uuid>,
) -> Result<EnterpriseInvitationsResponse, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .create_invitations(CreateInvitationsRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            emails,
            role: role.to_string(),
            workspace_ids: workspace_ids.into_iter().map(|id| id.to_string()).collect(),
            scope: scope_label(scope).to_string(),
            organization_id: organization_id(scope),
        })
        .await
        .map_err(invitation_grpc_error)?
        .into_inner();
    Ok(EnterpriseInvitationsResponse {
        invitations: response
            .invitations
            .into_iter()
            .map(invitation_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

fn invitation_grpc_error(error: tonic::Status) -> AppError {
    if error.code() == Code::AlreadyExists {
        return AppError::new(
            StatusCode::CONFLICT,
            "invitation_pending",
            "A pending invitation already exists for this email and workspace.",
        );
    }
    super::grpc_error(error)
}

fn invitation_from_grpc(value: GrpcInvitation) -> Result<EnterpriseInvitation, AppError> {
    let role = policy::role_from_db(&value.role);
    let module_grants = policy::grants_for_role(&role);
    Ok(EnterpriseInvitation {
        id: parse_uuid(&value.invitation_id, "invitation_id")?,
        email: value.email,
        role,
        module_grants,
        workspace_ids: value
            .workspace_ids
            .iter()
            .map(|workspace_id| parse_uuid(workspace_id, "workspace_id"))
            .collect::<Result<_, _>>()?,
        status: value.status,
        invited_at: parse_time(&value.invited_at, "invited_at")?,
        expires_at: optional_time(&value.expires_at, "expires_at")?,
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
            "enterprise_grpc_invalid_invitation",
            format!("{field} from Enterprise gRPC is invalid: {error}"),
        )
    })
}

fn parse_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|time| time.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_invitation",
                format!("{field} from Enterprise gRPC is invalid: {error}"),
            )
        })
}

fn optional_time(value: &str, field: &'static str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_time(value, field).map(Some)
    }
}
