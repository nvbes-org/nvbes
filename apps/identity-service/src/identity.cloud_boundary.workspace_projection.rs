use nvbes_product_identity::cloud_boundary::{
    AcceptWorkspaceInvitationCommand, CreateWorkspaceCommand, CreateWorkspaceInvitationCommand,
    UpdateWorkspaceMembershipRoleCommand, UpdateWorkspaceMembershipStatusCommand,
    UpdateWorkspaceSettingsCommand, UpsertWorkspaceMembershipCommand,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{cloud_boundary::workspace_invitation_record::InvitationRecord, http::error::AppError};

pub async fn project_create_workspace_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateWorkspaceCommand,
) -> Result<Uuid, AppError> {
    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, organization_id, workspace_type, name, plan_code,
          trial_ends_at, data_region, jurisdiction, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4::workspace_type, $5, $6, $7, $8::data_region, $9::legal_jurisdiction, $10, $10)
        "#,
    )
    .bind(command.workspace_id)
    .bind(command.tenant_id)
    .bind(command.organization_id)
    .bind(&command.workspace_type)
    .bind(&command.name)
    .bind(&command.plan_code)
    .bind(command.trial_ends_at)
    .bind(&command.data_region)
    .bind(&command.jurisdiction)
    .bind(command.created_at)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id, member_can_create_share_links,
          require_admin_approval_for_member_share, default_share_link_ttl_days,
          max_share_link_ttl_days, required_acr, mfa_policy, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, COALESCE($6::step_up_level, 'aal1'), COALESCE($7, 'optional'), $8, $8)
        "#,
    )
    .bind(command.workspace_id)
    .bind(command.policy.member_can_create_share_links)
    .bind(command.policy.require_admin_approval_for_member_share)
    .bind(command.policy.default_share_link_ttl_days)
    .bind(command.policy.max_share_link_ttl_days)
    .bind(command.policy.required_acr.as_deref())
    .bind(command.policy.mfa_policy.as_deref())
    .bind(command.created_at)
    .execute(&mut **tx)
    .await?;

    project_upsert_workspace_membership_tx(
        tx,
        command.workspace_id,
        command.owner_principal_id,
        &command.owner_role,
        "active",
        &command.membership_source,
    )
    .await?;

    Ok(command.workspace_id)
}

pub async fn project_update_workspace_settings_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpdateWorkspaceSettingsCommand,
) -> Result<(), AppError> {
    sqlx::query("UPDATE workspaces SET name = $2, updated_at = NOW() WHERE id = $1")
        .bind(command.workspace_id)
        .bind(&command.name)
        .execute(&mut **tx)
        .await?;

    sqlx::query(
        r#"
        UPDATE workspace_policies
        SET member_can_create_share_links = $2,
            require_admin_approval_for_member_share = $3,
            default_share_link_ttl_days = $4,
            max_share_link_ttl_days = $5,
            mfa_policy = COALESCE($6, mfa_policy),
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(command.workspace_id)
    .bind(command.policy.member_can_create_share_links)
    .bind(command.policy.require_admin_approval_for_member_share)
    .bind(command.policy.default_share_link_ttl_days)
    .bind(command.policy.max_share_link_ttl_days)
    .bind(command.policy.mfa_policy.as_deref())
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn project_upsert_membership_command_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpsertWorkspaceMembershipCommand,
) -> Result<(), AppError> {
    project_upsert_workspace_membership_tx(
        tx,
        command.workspace_id,
        command.principal_id,
        &command.role,
        &command.status,
        &command.source,
    )
    .await
}

pub async fn project_update_membership_role_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpdateWorkspaceMembershipRoleCommand,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET role = $3::workspace_member_role,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND principal_id = $2
        "#,
    )
    .bind(command.workspace_id)
    .bind(command.principal_id)
    .bind(&command.role)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn project_update_membership_status_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &UpdateWorkspaceMembershipStatusCommand,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET status = $3::workspace_member_status,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND principal_id = $2
        "#,
    )
    .bind(command.workspace_id)
    .bind(command.principal_id)
    .bind(&command.status)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn project_create_invitation_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreateWorkspaceInvitationCommand,
    invitation_id: Uuid,
) -> Result<InvitationRecord, AppError> {
    let invitation = sqlx::query_as::<_, InvitationRecord>(
        r#"
        INSERT INTO workspace_invitations (
          id, workspace_id, email, role, invited_by, token_hash, expires_at
        )
        VALUES ($1, $2, $3, $4::workspace_member_role, $5, $6, $7)
        RETURNING id, workspace_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
        "#,
    )
    .bind(invitation_id)
    .bind(command.workspace_id)
    .bind(&command.email)
    .bind(&command.role)
    .bind(command.invited_by)
    .bind(&command.token_hash)
    .bind(command.expires_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(invitation)
}

pub async fn project_accept_invitation_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &AcceptWorkspaceInvitationCommand,
) -> Result<(), AppError> {
    project_upsert_workspace_membership_tx(
        tx,
        command.workspace_id,
        command.principal_id,
        &command.role,
        "active",
        "invitation",
    )
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_invitations
        SET status = 'accepted', accepted_at = $2, updated_at = $2
        WHERE id = $1
        "#,
    )
    .bind(command.invitation_id)
    .bind(command.accepted_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn project_purge_principal_context_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), AppError> {
    for statement in [
        "DELETE FROM tenant_break_glass_accounts WHERE principal_id = $1",
        "DELETE FROM developer_token_debug_sessions WHERE actor_principal_id = $1",
        "DELETE FROM developer_role_assignments WHERE principal_id = $1",
        "DELETE FROM workspace_memberships WHERE principal_id = $1",
    ] {
        sqlx::query(statement)
            .bind(principal_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

async fn project_upsert_workspace_membership_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    principal_id: Uuid,
    role: &str,
    status: &str,
    source: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source)
        VALUES ($1, $2, $3::workspace_member_role, $4::workspace_member_status, $5::membership_source)
        ON CONFLICT (workspace_id, principal_id)
        DO UPDATE SET role = EXCLUDED.role, status = EXCLUDED.status, source = EXCLUDED.source, updated_at = NOW()
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(role)
    .bind(status)
    .bind(source)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
