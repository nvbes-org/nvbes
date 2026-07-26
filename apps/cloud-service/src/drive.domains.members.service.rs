use uuid::Uuid;

use crate::{
    domains::auth::types::AuthContext,
    domains::authz::{WorkspaceAccess, WorkspaceRole},
    http::error::AppError,
};
use nvbes_core::pagination::page_from_rows;
use nvbes_tenancy::role_as_db;

use super::db;
use super::db::AuditEventInput;
use super::invitations;
use super::pagination::{InvitationListCursor, MemberListCursor, normalize_email_prefix};

#[cfg(test)]
#[path = "drive.domains.members.service.db_tests.rs"]
mod db_tests;
pub use super::types::{
    AcceptInvitationInput, AcceptInvitationResponse, InvitationListResponse, InviteMemberInput,
    InviteMemberResponse, ListMembersInput, MemberListResponse, RemoveMemberResponse,
    UpdateMemberInput, UpdateMemberResponse,
};

pub async fn list_members(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: ListMembersInput,
) -> Result<MemberListResponse, AppError> {
    let limit = input.limit.unwrap_or(50).clamp(1, 200);
    let cursor = input
        .cursor
        .as_deref()
        .map(MemberListCursor::decode)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let email_prefix = normalize_email_prefix(input.email_prefix);
    let members = db::list_members(
        db,
        access.workspace_id,
        cursor.as_ref(),
        input.role.map(role_as_db),
        email_prefix.as_deref(),
        limit + 1,
    )
    .await?;
    let page = page_from_rows(members, limit as usize, MemberListCursor::from_record);
    Ok(MemberListResponse {
        members: page
            .items
            .into_iter()
            .map(db::MemberRecord::into_view)
            .collect(),
        members_next_cursor: page
            .next_cursor
            .as_ref()
            .map(MemberListCursor::encode)
            .transpose()
            .map_err(|_| {
                AppError::internal(
                    "cursor_encoding_failed",
                    "Pagination cursor could not be encoded.",
                )
            })?,
        members_has_more: page.has_more,
    })
}

pub async fn list_invitations(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    limit: Option<i64>,
    cursor: Option<String>,
    statuses: Vec<String>,
) -> Result<InvitationListResponse, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let cursor = cursor
        .as_deref()
        .map(InvitationListCursor::decode)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let normalized_statuses = statuses
        .into_iter()
        .map(|status| status.trim().to_lowercase())
        .filter(|status| !status.is_empty())
        .collect::<Vec<_>>();
    let invitations = db::list_invitations(
        db,
        access.workspace_id,
        cursor.as_ref(),
        &normalized_statuses,
        limit + 1,
    )
    .await?;
    let page = page_from_rows(
        invitations,
        limit as usize,
        InvitationListCursor::from_record,
    );

    Ok(InvitationListResponse {
        invitations: page
            .items
            .into_iter()
            .map(db::InvitationRecord::into_view)
            .collect(),
        next_cursor: page
            .next_cursor
            .as_ref()
            .map(InvitationListCursor::encode)
            .transpose()
            .map_err(|_| {
                AppError::internal(
                    "cursor_encoding_failed",
                    "Pagination cursor could not be encoded.",
                )
            })?,
        has_more: page.has_more,
    })
}

pub async fn invite_member(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: InviteMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<InviteMemberResponse, AppError> {
    invitations::invite_member(db, access, input, ip, user_agent).await
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
