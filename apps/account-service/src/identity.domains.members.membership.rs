use uuid::Uuid;

use crate::{
    domains::{authz::WorkspaceAccess, cloud::workspace_port},
    http::error::AppError,
};
use nvbes_product_account::cloud_boundary::{
    UpdateWorkspaceMembershipRoleCommand, UpdateWorkspaceMembershipStatusCommand,
};
use nvbes_tenancy::role_as_db;
use sqlx::{PgPool, Row};

use super::invites::InvitationRecord;
use super::records::MemberRecord;
use super::types::*;
use crate::domains::auth;

pub async fn list_members(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<MemberListResponse, AppError> {
    let mut members = Vec::new();
    for member in workspace_port::list_workspace_members(
        access.tenant_id,
        access.workspace_id,
        access.auth.user_id,
    )
    .await?
    {
        if let Some(record) = member_record_from_cloud_member(db, member).await? {
            members.push(record);
        }
    }
    members.sort_by(|left, right| {
        member_role_rank(&left.role)
            .cmp(&member_role_rank(&right.role))
            .then_with(|| left.email.cmp(&right.email))
    });

    let invitations = workspace_port::list_workspace_invitations(
        access.tenant_id,
        access.workspace_id,
        access.auth.user_id,
        &["pending", "accepted"],
    )
    .await?;

    Ok(MemberListResponse {
        members: members.into_iter().map(MemberRecord::into_view).collect(),
        invitations: invitations
            .into_iter()
            .map(InvitationRecord::into_view)
            .collect(),
    })
}

pub async fn update_member_role(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    member_id: Uuid,
    input: UpdateMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<UpdateMemberResponse, AppError> {
    if matches!(input.role, crate::domains::authz::WorkspaceRole::Owner) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Owner role cannot be assigned through role changes.",
        ));
    }

    let current_member = active_member_record(db, access, member_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("member_not_found", "Active member not found in workspace.")
        })?;

    if current_member.role == role_as_db(input.role) {
        return Err(AppError::conflict(
            "member_role_unchanged",
            "The member already has this role.",
        ));
    }

    let mut tx = db.begin().await?;

    crate::domains::cloud::workspace_port::update_workspace_membership_role_tx(
        &mut tx,
        &UpdateWorkspaceMembershipRoleCommand {
            actor_principal_id: access.auth.user_id,
            workspace_id: access.workspace_id,
            principal_id: member_id,
            role: role_as_db(input.role).to_string(),
        },
    )
    .await?;
    let updated_member = active_member_record(db, access, member_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("member_not_found", "Active member not found in workspace.")
        })?;

    auth::sessions::revoke_all_user_sessions_tx(&mut tx, member_id).await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: access.tenant_id.unwrap_or_default(),
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
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
    nvbes_redis::session::clear_user_sessions(redis, &member_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))?;

    Ok(UpdateMemberResponse {
        member: updated_member.into_view(),
        sessions_revoked: true,
    })
}

pub async fn remove_member(
    db: &PgPool,
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

    if active_member_record(db, access, member_id).await?.is_none() {
        return Err(AppError::not_found(
            "member_not_found",
            "Active member not found in workspace.",
        ));
    }

    let mut tx = db.begin().await?;

    crate::domains::cloud::workspace_port::update_workspace_membership_status_tx(
        &mut tx,
        &UpdateWorkspaceMembershipStatusCommand {
            actor_principal_id: access.auth.user_id,
            workspace_id: access.workspace_id,
            principal_id: member_id,
            status: "removed".to_string(),
        },
    )
    .await?;

    auth::sessions::revoke_all_user_sessions_tx(&mut tx, member_id).await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: access.tenant_id.unwrap_or_default(),
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
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
    nvbes_redis::session::clear_user_sessions(redis, &member_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))?;

    Ok(RemoveMemberResponse {
        removed_user_id: member_id,
        sessions_revoked: true,
    })
}

async fn active_member_record(
    db: &PgPool,
    access: &WorkspaceAccess,
    member_id: Uuid,
) -> Result<Option<MemberRecord>, AppError> {
    let member = workspace_port::list_workspace_members(
        access.tenant_id,
        access.workspace_id,
        access.auth.user_id,
    )
    .await?
    .into_iter()
    .find(|member| member.principal_id == member_id && member.active);

    match member {
        Some(member) => member_record_from_cloud_member(db, member).await,
        None => Ok(None),
    }
}

async fn member_record_from_cloud_member(
    db: &PgPool,
    member: workspace_port::CloudWorkspaceMemberSummary,
) -> Result<Option<MemberRecord>, AppError> {
    let user = sqlx::query(
        r#"
        SELECT email, firstname, lastname, username
        FROM users
        WHERE principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(member.principal_id)
    .fetch_optional(db)
    .await?;
    let Some(user) = user else {
        return Ok(None);
    };

    Ok(Some(MemberRecord {
        user_id: member.principal_id,
        email: user.get("email"),
        firstname: user.get("firstname"),
        lastname: user.get("lastname"),
        username: user.get("username"),
        role: member.role,
        status: member.status,
        created_at: member.joined_at,
        updated_at: member.updated_at,
    }))
}

fn member_role_rank(role: &str) -> i32 {
    match role {
        "owner" => 0,
        "admin" => 1,
        "member" => 2,
        "viewer" => 3,
        _ => 4,
    }
}
