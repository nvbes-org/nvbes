use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use nvbes_product_account::cloud_boundary::{
    AcceptWorkspaceInvitationCommand, CreateWorkspaceInvitationCommand,
};
use uuid::Uuid;

use crate::domains::auth::types::AuthContext;
use crate::domains::authz::WorkspaceAccess;
use crate::domains::{auth, cloud::workspace_port};
use crate::http::error::AppError;
use nvbes_core::config::AppConfig;
use nvbes_tenancy::role_as_db;
use sqlx::PgPool;

use crate::domains::members::types::*;

pub async fn invite_member(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: InviteMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<InviteMemberResponse, AppError> {
    ensure_invitable_role(input.role)?;
    let email = auth::password::normalize_email(&input.email);
    auth::password::validate_email(&email)?;
    let inviter = auth::db::fetch_user_record(db, access.auth.user_id).await?;
    if email == auth::password::normalize_email(&inviter.email) {
        return Err(AppError::conflict(
            "member_conflict",
            "You are already a member of this workspace.",
        ));
    }

    let existing_member = active_member_exists_for_email(db, access, &email).await?;
    if existing_member {
        return Err(AppError::conflict(
            "member_conflict",
            "This user is already an active member of the workspace.",
        ));
    }
    let pending_invitation = pending_invitation_exists(access, &email, Utc::now()).await?;
    if pending_invitation {
        return Err(AppError::conflict(
            "invitation_pending",
            "A pending invitation already exists for this email.",
        ));
    }
    let workspace =
        workspace_port::get_workspace(access.tenant_id, access.workspace_id, access.auth.user_id)
            .await?;

    let mut tx = db.begin().await?;
    let invitation_token = auth::password::generate_token("gxi");
    auth::password::log_dev_token(&invitation_token, &config.environment, "member_invitation");
    let token_hash = auth::password::token_hash(&invitation_token);
    let expires_at = Utc::now() + Duration::days(7);
    let invitation = workspace_port::create_workspace_invitation_tx(
        &mut tx,
        &CreateWorkspaceInvitationCommand {
            workspace_id: access.workspace_id,
            email: email.clone(),
            role: role_as_db(input.role).to_string(),
            invited_by: access.auth.user_id,
            token_hash,
            expires_at,
        },
    )
    .await?;
    let email_msg = crate::email::templates::invitation_email(
        config,
        &email,
        &workspace.name,
        &inviter.display_name,
        &invitation_token,
    )?;
    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: access.tenant_id.unwrap_or_default(),
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
            action: "member.invited",
            target_type: "workspace_invitation",
            target_id: Some(invitation.id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({"email": invitation.email, "role": invitation.role}),
        },
    )
    .await?;
    tx.commit().await?;

    crate::email::jobs::enqueue_email_job_tx(
        db,
        redis,
        crate::email::jobs::EmailSendPayload {
            to_email: email.clone(),
            to_name: None,
            subject: email_msg.subject,
            html_body: email_msg.html_body.unwrap_or_default(),
            text_body: email_msg.text_body,
            business_type: "invitation".to_string(),
        },
        &format!("invite:{}", invitation.id),
    )
    .await?;
    Ok(InviteMemberResponse {
        invitation: invitation.into_view(),
    })
}

pub async fn accept_invitation(
    db: &PgPool,
    auth: &AuthContext,
    input: AcceptInvitationInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<AcceptInvitationResponse, AppError> {
    let token_hash_value = auth::password::token_hash(input.token.trim());
    let now = Utc::now();
    let invitation = match workspace_port::get_workspace_invitation_by_token_hash(
        &token_hash_value,
        auth.user_id,
    )
    .await
    {
        Err(error) if error.status == StatusCode::NOT_FOUND => {
            return Err(AppError::bad_request(
                "invalid_invitation",
                "Invitation token is invalid.",
            ));
        }
        result => result?,
    };
    let invitation_id = invitation.id;
    let workspace_id = invitation.workspace_id;
    let email = invitation.email;
    let role_string = invitation.role;
    let status = invitation.status;
    let expires_at = invitation.expires_at;
    let accepted_at = invitation.accepted_at;
    let current_email = auth::db::fetch_user_record(db, auth.user_id).await?.email;
    if auth::password::normalize_email(&email) != auth::password::normalize_email(&current_email) {
        return Err(AppError::forbidden(
            "invitation_email_mismatch",
            "This invitation was issued to another email address.",
        ));
    }
    if status != "pending" || accepted_at.is_some() || expires_at < now {
        return Err(AppError::bad_request(
            "invitation_unavailable",
            "Invitation is no longer available.",
        ));
    }
    let workspace =
        workspace_port::get_workspace(auth.tenant_id, workspace_id, auth.user_id).await?;
    let existing = active_member_exists(
        Some(workspace.tenant_id),
        workspace_id,
        auth.user_id,
        auth.user_id,
    )
    .await?;
    if existing {
        return Err(AppError::conflict(
            "member_conflict",
            "You are already an active member of this workspace.",
        ));
    }
    let mut tx = db.begin().await?;
    workspace_port::accept_workspace_invitation_tx(
        &mut tx,
        &AcceptWorkspaceInvitationCommand {
            invitation_id,
            workspace_id,
            principal_id: auth.user_id,
            role: role_string.clone(),
            accepted_at: now,
        },
    )
    .await?;
    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: auth.tenant_id.unwrap_or(workspace.tenant_id),
            workspace_id: Some(workspace_id),
            actor_principal_id: Some(auth.user_id),
            action: "member.accepted",
            target_type: "workspace_invitation",
            target_id: Some(invitation_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({ "role": role_string }),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(AcceptInvitationResponse {
        workspace_id,
        role: role_string,
        accepted_at: now,
    })
}

fn ensure_invitable_role(role: crate::domains::authz::WorkspaceRole) -> Result<(), AppError> {
    if matches!(role, crate::domains::authz::WorkspaceRole::Owner) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Owner role cannot be assigned through invitations.",
        ));
    }

    Ok(())
}

async fn active_member_exists_for_email(
    db: &PgPool,
    access: &WorkspaceAccess,
    email: &str,
) -> Result<bool, AppError> {
    let Some(principal_id) = user_principal_id_by_email(db, email).await? else {
        return Ok(false);
    };

    active_member_exists(
        access.tenant_id,
        access.workspace_id,
        access.auth.user_id,
        principal_id,
    )
    .await
}

async fn user_principal_id_by_email(db: &PgPool, email: &str) -> Result<Option<Uuid>, AppError> {
    sqlx::query_scalar::<_, Uuid>("SELECT principal_id FROM users WHERE lower(email) = $1 LIMIT 1")
        .bind(email)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)
}

async fn active_member_exists(
    tenant_id: Option<Uuid>,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    principal_id: Uuid,
) -> Result<bool, AppError> {
    let members =
        workspace_port::list_workspace_members(tenant_id, workspace_id, actor_principal_id).await?;

    Ok(members
        .into_iter()
        .any(|member| member.principal_id == principal_id && member.active))
}

async fn pending_invitation_exists(
    access: &WorkspaceAccess,
    email: &str,
    now: DateTime<Utc>,
) -> Result<bool, AppError> {
    let invitations = workspace_port::list_workspace_invitations(
        access.tenant_id,
        access.workspace_id,
        access.auth.user_id,
        &["pending"],
    )
    .await?;

    Ok(invitations.into_iter().any(|invitation| {
        auth::password::normalize_email(&invitation.email) == email && invitation.expires_at > now
    }))
}

#[cfg(test)]
mod tests {
    use super::ensure_invitable_role;
    use crate::domains::authz::WorkspaceRole;

    #[test]
    fn invite_member_rejects_owner_role() {
        let error = ensure_invitable_role(WorkspaceRole::Owner)
            .expect_err("owner role should not be assignable");

        assert_eq!(error.code, "validation_failed");
    }

    #[test]
    fn invite_member_allows_non_owner_roles() {
        assert!(ensure_invitable_role(WorkspaceRole::Member).is_ok());
    }
}
