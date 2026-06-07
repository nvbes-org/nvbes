use uuid::Uuid;

use crate::{
    domains::auth::types::AuthContext,
    domains::authz::{WorkspaceAccess, WorkspaceRole},
    http::error::AppError,
};
use nvbes_audit::AuditEventInput;
use nvbes_core::config::AppConfig;
use nvbes_tenancy::role_as_db;

use super::db;
use super::invitations;
pub use super::types::{
    AcceptInvitationInput, AcceptInvitationResponse, InviteMemberInput, InviteMemberResponse,
    MemberListResponse, RemoveMemberResponse, UpdateMemberInput, UpdateMemberResponse,
};

pub async fn list_members(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
) -> Result<MemberListResponse, AppError> {
    let members = db::list_members(db, access.workspace_id).await?;
    let invitations = db::list_pending_invitations(db, access.workspace_id).await?;

    Ok(MemberListResponse {
        members: members
            .into_iter()
            .map(db::MemberRecord::into_view)
            .collect(),
        invitations: invitations
            .into_iter()
            .map(db::InvitationRecord::into_view)
            .collect(),
    })
}

pub async fn invite_member(
    db: &sqlx::PgPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: InviteMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<InviteMemberResponse, AppError> {
    invitations::invite_member(db, config, access, input, ip, user_agent).await
}

pub async fn update_member_role(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    member_id: Uuid,
    input: UpdateMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<UpdateMemberResponse, AppError> {
    if matches!(input.role, WorkspaceRole::Owner) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Owner role cannot be assigned through role changes.",
        ));
    }

    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let current_member =
        db::fetch_active_member_tx(&mut tx, access.workspace_id, member_id).await?;

    let current_member = current_member.ok_or_else(|| {
        AppError::not_found("member_not_found", "Active member not found in workspace.")
    })?;

    if current_member.role == role_as_db(input.role) {
        return Err(AppError::conflict(
            "member_role_unchanged",
            "The member already has this role.",
        ));
    }

    let updated_member = db::update_member_role_tx(
        &mut tx,
        access.workspace_id,
        member_id,
        role_as_db(input.role),
    )
    .await?;

    db::insert_audit_event(
        &mut tx,
        AuditEventInput {
            tenant_id: access.tenant_id.ok_or_else(|| {
                AppError::internal("missing_tenant", "Tenant context is required.")
            })?,
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.principal_id),
            action: "member.role_changed",
            target_type: "workspace_member",
            target_id: Some(member_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
              "previous_role": current_member.role,
              "new_role": updated_member.role
            }),
        },
    )
    .await?;

    tx.commit().await?;
    db::revoke_user_sessions(redis, member_id).await?;

    Ok(UpdateMemberResponse {
        member: updated_member.into_view(),
        sessions_revoked: true,
    })
}

pub async fn remove_member(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    member_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<RemoveMemberResponse, AppError> {
    if member_id == access.auth.user_id {
        return Err(AppError::bad_request(
            "validation_failed",
            "Owners must transfer ownership before leaving; self-removal is not supported.",
        ));
    }

    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let existing = db::fetch_active_member_tx(&mut tx, access.workspace_id, member_id).await?;

    if existing.is_none() {
        return Err(AppError::not_found(
            "member_not_found",
            "Active member not found in workspace.",
        ));
    }

    db::remove_member_tx(&mut tx, access.workspace_id, member_id).await?;

    db::insert_audit_event(
        &mut tx,
        AuditEventInput {
            tenant_id: access.tenant_id.ok_or_else(|| {
                AppError::internal("missing_tenant", "Tenant context is required.")
            })?,
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.principal_id),
            action: "member.removed",
            target_type: "workspace_member",
            target_id: Some(member_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({}),
        },
    )
    .await?;

    tx.commit().await?;
    db::revoke_user_sessions(redis, member_id).await?;

    Ok(RemoveMemberResponse {
        removed_user_id: member_id,
        sessions_revoked: true,
    })
}

pub async fn accept_invitation(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    input: AcceptInvitationInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<AcceptInvitationResponse, AppError> {
    invitations::accept_invitation(db, auth, input, ip, user_agent).await
}
