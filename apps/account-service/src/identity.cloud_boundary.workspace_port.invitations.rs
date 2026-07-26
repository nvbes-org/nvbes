use uuid::Uuid;

use crate::{
    cloud_boundary::workspace_invitation_record::InvitationRecord,
    grpc_pb::nvbes::cloud::v1::{
        GetWorkspaceInvitationByTokenHashRequest, ListWorkspaceInvitationsRequest,
        WorkspaceInvitation,
    },
    http::error::AppError,
};

use super::{
    cloud_client, grpc_error, request_context,
    workspace_port_parse::{parse_datetime, parse_optional_datetime, parse_uuid},
};

pub async fn list_workspace_invitations(
    tenant_id: Option<Uuid>,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    statuses: &[&str],
) -> Result<Vec<InvitationRecord>, AppError> {
    let mut client = cloud_client().await?;
    let response = client
        .list_workspace_invitations(ListWorkspaceInvitationsRequest {
            context: Some(request_context(
                tenant_id,
                workspace_id,
                actor_principal_id,
                "",
            )),
            workspace_id: workspace_id.to_string(),
            statuses: statuses
                .iter()
                .map(|status| (*status).to_string())
                .collect(),
            page: None,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .invitations
        .into_iter()
        .map(invitation_record)
        .collect()
}

pub async fn get_workspace_invitation_by_token_hash(
    token_hash: &str,
    actor_principal_id: Uuid,
) -> Result<InvitationRecord, AppError> {
    let mut client = cloud_client().await?;
    let invitation = client
        .get_workspace_invitation_by_token_hash(GetWorkspaceInvitationByTokenHashRequest {
            context: Some(request_context(None, Uuid::nil(), actor_principal_id, "")),
            token_hash: token_hash.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    invitation_record(invitation)
}

fn invitation_record(invitation: WorkspaceInvitation) -> Result<InvitationRecord, AppError> {
    Ok(InvitationRecord {
        id: parse_uuid(&invitation.invitation_id, "invitation_id")?,
        workspace_id: parse_uuid(&invitation.workspace_id, "workspace_id")?,
        email: invitation.email,
        role: invitation.role,
        status: invitation.status,
        expires_at: parse_datetime(&invitation.expires_at, "expires_at")?,
        accepted_at: parse_optional_datetime(&invitation.accepted_at, "accepted_at")?,
        revoked_at: parse_optional_datetime(&invitation.revoked_at, "revoked_at")?,
        created_at: parse_datetime(&invitation.created_at, "created_at")?,
    })
}
