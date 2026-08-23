use nvbes_product_identity::cloud_boundary::{
    AcceptWorkspaceInvitationCommand, CreateWorkspaceCommand, CreateWorkspaceInvitationCommand,
    UpdateWorkspaceMembershipRoleCommand, UpdateWorkspaceMembershipStatusCommand,
    UpdateWorkspaceSettingsCommand, UpsertWorkspaceMembershipCommand, WorkspacePolicyCommand,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    cloud_boundary::{workspace_invitation_record::InvitationRecord, workspace_projection},
    grpc_pb::nvbes::cloud::v1::{
        AcceptWorkspaceInvitationRequest, AddWorkspaceMemberRequest,
        CreateWorkspaceInvitationRequest, CreateWorkspaceRequest, UpdateWorkspaceMemberRequest,
        UpdateWorkspaceRequest, WorkspacePolicy,
    },
    http::error::AppError,
};

use super::{cloud_client, grpc_error, principal_projection_tx, request_context};

pub async fn create_workspace_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateWorkspaceCommand,
) -> Result<Uuid, AppError> {
    let owner_principal = principal_projection_tx(tx, command.owner_principal_id).await?;
    let mut client = cloud_client().await?;
    client
        .create_workspace(CreateWorkspaceRequest {
            context: Some(request_context(
                Some(command.tenant_id),
                command.workspace_id,
                command.owner_principal_id,
                &command.data_region,
            )),
            name: command.name.clone(),
            region_id: command.data_region.clone(),
            data_residency: command.data_region.clone(),
            workspace_id: command.workspace_id.to_string(),
            tenant_id: command.tenant_id.to_string(),
            organization_id: command
                .organization_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            owner_principal_id: command.owner_principal_id.to_string(),
            workspace_type: command.workspace_type.clone(),
            plan_code: command.plan_code.clone(),
            trial_ends_at: command
                .trial_ends_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
            legal_jurisdiction: command.jurisdiction.clone(),
            owner_role: command.owner_role.clone(),
            membership_source: command.membership_source.clone(),
            policy: Some(workspace_policy(&command.policy)),
            created_at: command.created_at.to_rfc3339(),
            owner_principal: Some(owner_principal),
        })
        .await
        .map_err(grpc_error)?;

    workspace_projection::project_create_workspace_tx(tx, command).await
}

pub async fn update_workspace_settings_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpdateWorkspaceSettingsCommand,
) -> Result<(), AppError> {
    let mut client = cloud_client().await?;
    client
        .update_workspace(UpdateWorkspaceRequest {
            context: Some(request_context(
                None,
                command.workspace_id,
                command.actor_principal_id,
                "",
            )),
            workspace_id: command.workspace_id.to_string(),
            name: command.name.clone(),
            default_role: String::new(),
            policy: Some(workspace_policy(&command.policy)),
        })
        .await
        .map_err(grpc_error)?;

    workspace_projection::project_update_workspace_settings_tx(tx, command).await
}

pub async fn upsert_workspace_membership_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpsertWorkspaceMembershipCommand,
) -> Result<(), AppError> {
    let principal = principal_projection_tx(tx, command.principal_id).await?;
    let mut client = cloud_client().await?;
    client
        .add_workspace_member(AddWorkspaceMemberRequest {
            context: Some(request_context(
                None,
                command.workspace_id,
                command.actor_principal_id,
                "",
            )),
            workspace_id: command.workspace_id.to_string(),
            principal_id: command.principal_id.to_string(),
            role: command.role.clone(),
            source: command.source.clone(),
            principal: Some(principal),
        })
        .await
        .map_err(grpc_error)?;

    workspace_projection::project_upsert_membership_command_tx(tx, command).await
}

pub async fn update_workspace_membership_role_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpdateWorkspaceMembershipRoleCommand,
) -> Result<(), AppError> {
    let principal = principal_projection_tx(tx, command.principal_id).await?;
    let mut client = cloud_client().await?;
    client
        .update_workspace_member(UpdateWorkspaceMemberRequest {
            context: Some(request_context(
                None,
                command.workspace_id,
                command.actor_principal_id,
                "",
            )),
            workspace_id: command.workspace_id.to_string(),
            principal_id: command.principal_id.to_string(),
            role: command.role.clone(),
            status: String::new(),
            source: String::new(),
            principal: Some(principal),
        })
        .await
        .map_err(grpc_error)?;

    workspace_projection::project_update_membership_role_tx(tx, command).await
}

pub async fn update_workspace_membership_status_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpdateWorkspaceMembershipStatusCommand,
) -> Result<(), AppError> {
    let principal = principal_projection_tx(tx, command.principal_id).await?;
    let mut client = cloud_client().await?;
    client
        .update_workspace_member(UpdateWorkspaceMemberRequest {
            context: Some(request_context(
                None,
                command.workspace_id,
                command.actor_principal_id,
                "",
            )),
            workspace_id: command.workspace_id.to_string(),
            principal_id: command.principal_id.to_string(),
            role: String::new(),
            status: command.status.clone(),
            source: String::new(),
            principal: Some(principal),
        })
        .await
        .map_err(grpc_error)?;

    workspace_projection::project_update_membership_status_tx(tx, command).await
}

pub async fn create_workspace_invitation_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateWorkspaceInvitationCommand,
) -> Result<InvitationRecord, AppError> {
    let invited_by = principal_projection_tx(tx, command.invited_by).await?;
    let mut client = cloud_client().await?;
    let invitation = client
        .create_workspace_invitation(CreateWorkspaceInvitationRequest {
            context: Some(request_context(
                None,
                command.workspace_id,
                command.invited_by,
                "",
            )),
            workspace_id: command.workspace_id.to_string(),
            email: command.email.clone(),
            role: command.role.clone(),
            invited_by_principal_id: command.invited_by.to_string(),
            token_hash: command.token_hash.clone(),
            expires_at: command.expires_at.to_rfc3339(),
            invited_by_principal: Some(invited_by),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    let invitation_id = Uuid::parse_str(&invitation.invitation_id).map_err(|_| {
        AppError::internal(
            "cloud_grpc_invalid_response",
            "Cloud returned an invalid invitation id.",
        )
    })?;

    workspace_projection::project_create_invitation_tx(tx, command, invitation_id).await
}

pub async fn accept_workspace_invitation_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &AcceptWorkspaceInvitationCommand,
) -> Result<(), AppError> {
    let principal = principal_projection_tx(tx, command.principal_id).await?;
    let mut client = cloud_client().await?;
    client
        .accept_workspace_invitation(AcceptWorkspaceInvitationRequest {
            context: Some(request_context(
                None,
                command.workspace_id,
                command.principal_id,
                "",
            )),
            invitation_id: command.invitation_id.to_string(),
            workspace_id: command.workspace_id.to_string(),
            principal_id: command.principal_id.to_string(),
            role: command.role.clone(),
            accepted_at: command.accepted_at.to_rfc3339(),
            principal: Some(principal),
        })
        .await
        .map_err(grpc_error)?;

    workspace_projection::project_accept_invitation_tx(tx, command).await
}

fn workspace_policy(value: &WorkspacePolicyCommand) -> WorkspacePolicy {
    WorkspacePolicy {
        member_can_create_share_links: value.member_can_create_share_links,
        require_admin_approval_for_member_share: value.require_admin_approval_for_member_share,
        default_share_link_ttl_days: value.default_share_link_ttl_days,
        max_share_link_ttl_days: value.max_share_link_ttl_days,
        required_acr: value.required_acr.clone().unwrap_or_default(),
        mfa_policy: value.mfa_policy.clone().unwrap_or_default(),
    }
}
