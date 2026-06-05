use uuid::Uuid;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_audit::AuditEventInput;
use nvbes_tenancy::role_as_db;
use sqlx::PgPool;

use super::invites::InvitationRecord;
use super::records::MemberRecord;
use super::types::*;
use crate::domains::auth;

pub async fn list_members(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<MemberListResponse, AppError> {
    let members = sqlx::query_as::<_, MemberRecord>(
        r#"
        SELECT
          wm.principal_id AS user_id,
          u.email,
          u.firstname,
          u.lastname,
          u.username,
          wm.role::text AS role,
          wm.status::text AS status,
          wm.created_at,
          wm.updated_at
        FROM workspace_memberships wm
        INNER JOIN users u ON u.principal_id = wm.principal_id
        WHERE wm.workspace_id = $1
        ORDER BY
          CASE wm.role
            WHEN 'owner' THEN 0
            WHEN 'admin' THEN 1
            WHEN 'member' THEN 2
            WHEN 'viewer' THEN 3
          END,
          u.email ASC
        "#,
    )
    .bind(access.workspace_id)
    .fetch_all(db)
    .await?;

    let invitations = sqlx::query_as::<_, InvitationRecord>(
        r#"
        SELECT
          id,
          email,
          role::text AS role,
          status::text AS status,
          expires_at,
          accepted_at,
          revoked_at,
          created_at
        FROM workspace_invitations
        WHERE workspace_id = $1
          AND status IN ('pending', 'accepted')
        ORDER BY created_at DESC
        "#,
    )
    .bind(access.workspace_id)
    .fetch_all(db)
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

    let mut tx = db.begin().await?;

    let current_member = sqlx::query_as::<_, MemberRecord>(
        r#"
        SELECT
          wm.principal_id AS user_id,
          u.email,
          u.firstname,
          u.lastname,
          u.username,
          wm.role::text AS role,
          wm.status::text AS status,
          wm.created_at,
          wm.updated_at
        FROM workspace_memberships wm
        INNER JOIN users u ON u.principal_id = wm.principal_id
        WHERE wm.workspace_id = $1
          AND wm.principal_id = $2
          AND wm.status = 'active'
        "#,
    )
    .bind(access.workspace_id)
    .bind(member_id)
    .fetch_optional(&mut *tx)
    .await?;

    let current_member = current_member.ok_or_else(|| {
        AppError::not_found("member_not_found", "Active member not found in workspace.")
    })?;

    if current_member.role == role_as_db(input.role) {
        return Err(AppError::conflict(
            "member_role_unchanged",
            "The member already has this role.",
        ));
    }

    let updated_member = sqlx::query_as::<_, MemberRecord>(
        r#"
        UPDATE workspace_memberships
        SET role = $3::workspace_member_role,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND principal_id = $2
        RETURNING
          principal_id AS user_id,
          (
            SELECT email FROM users WHERE principal_id = workspace_memberships.principal_id
          ) AS email,
          (
            SELECT firstname FROM users WHERE principal_id = workspace_memberships.principal_id
          ) AS firstname,
          (
            SELECT lastname FROM users WHERE principal_id = workspace_memberships.principal_id
          ) AS lastname,
          (
            SELECT username FROM users WHERE principal_id = workspace_memberships.principal_id
          ) AS username,
          role::text AS role,
          status::text AS status,
          created_at,
          updated_at
        "#,
    )
    .bind(access.workspace_id)
    .bind(member_id)
    .bind(role_as_db(input.role))
    .fetch_one(&mut *tx)
    .await?;

    auth::sessions::revoke_all_user_sessions_tx(&mut tx, member_id).await?;

    nvbes_audit::insert_audit_event_tx(
        &mut tx,
        AuditEventInput {
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
        .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;

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

    let mut tx = db.begin().await?;

    let existing = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships
        WHERE workspace_id = $1
          AND principal_id = $2
          AND status = 'active'
        "#,
    )
    .bind(access.workspace_id)
    .bind(member_id)
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
        return Err(AppError::not_found(
            "member_not_found",
            "Active member not found in workspace.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET status = 'removed',
            updated_at = NOW()
        WHERE workspace_id = $1
          AND principal_id = $2
        "#,
    )
    .bind(access.workspace_id)
    .bind(member_id)
    .execute(&mut *tx)
    .await?;

    auth::sessions::revoke_all_user_sessions_tx(&mut tx, member_id).await?;

    nvbes_audit::insert_audit_event_tx(
        &mut tx,
        AuditEventInput {
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
        .map_err(|err| AppError::internal("redis_session_revoke_failed", &err.to_string()))?;

    Ok(RemoveMemberResponse {
        removed_user_id: member_id,
        sessions_revoked: true,
    })
}
