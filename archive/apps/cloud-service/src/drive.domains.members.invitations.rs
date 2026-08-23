use chrono::{DateTime, Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    domains::auth::types::AuthContext,
    domains::authz::{WorkspaceAccess, WorkspaceRole},
    http::error::AppError,
};
use nvbes_tenancy::role_as_db;

use super::db::*;
use super::types::*;
use nvbes_core::auth::{generate_token, normalize_email, token_hash, validate_email};

pub async fn invite_member(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: InviteMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<InviteMemberResponse, AppError> {
    if matches!(input.role, WorkspaceRole::Owner) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Owner role cannot be assigned through invitations.",
        ));
    }

    let email = normalize_email(&input.email);
    validate_email(&email)?;

    let current_email = fetch_current_user_email(db, access.auth.user_id).await?;
    if email == normalize_email(&current_email) {
        return Err(AppError::conflict(
            "member_conflict",
            "You are already a member of this workspace.",
        ));
    }

    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let existing_member = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships wm
        INNER JOIN users u ON u.id = wm.user_id
        WHERE wm.workspace_id = $1
          AND lower(u.email) = $2
          AND wm.status = 'active'
        "#,
    )
    .bind(access.workspace_id)
    .bind(&email)
    .fetch_one(&mut *tx)
    .await?;

    if existing_member > 0 {
        return Err(AppError::conflict(
            "member_conflict",
            "This user is already an active member of the workspace.",
        ));
    }

    let pending_invitation = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_invitations
        WHERE workspace_id = $1
          AND email = $2
          AND status = 'pending'
          AND expires_at > NOW()
        "#,
    )
    .bind(access.workspace_id)
    .bind(&email)
    .fetch_one(&mut *tx)
    .await?;

    if pending_invitation > 0 {
        return Err(AppError::conflict(
            "invitation_pending",
            "A pending invitation already exists for this email.",
        ));
    }

    let invitation_token = generate_token("gxi");
    let token_hash = token_hash(&invitation_token);
    let expires_at = Utc::now() + Duration::days(7);

    let invitation = sqlx::query_as::<_, InvitationRecord>(
        r#"
        INSERT INTO workspace_invitations (
          workspace_id,
          email,
          role,
          invited_by,
          token_hash,
          expires_at
        )
        VALUES ($1, $2, $3::workspace_member_role, $4, $5, $6)
        RETURNING
          id,
          email,
          role::text AS role,
          status::text AS status,
          expires_at,
          accepted_at,
          revoked_at,
          created_at
        "#,
    )
    .bind(access.workspace_id)
    .bind(&email)
    .bind(role_as_db(input.role))
    .bind(access.auth.user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;

    insert_audit_event(
        &mut tx,
        AuditEventInput {
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.principal_id),
            action: "member.invited",
            target_type: "workspace_invitation",
            target_id: Some(invitation.id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
              "email": invitation.email,
              "role": invitation.role
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(InviteMemberResponse {
        invitation: invitation.into_view(),
    })
}

pub async fn accept_invitation(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    input: AcceptInvitationInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<AcceptInvitationResponse, AppError> {
    let token_hash_value = token_hash(input.token.trim());
    let now = Utc::now();
    let mut tx = db.begin().await?;

    let invitation = sqlx::query(
        r#"
        SELECT
          id,
          workspace_id,
          email,
          role::text AS role,
          status::text AS status,
          expires_at,
          accepted_at
        FROM workspace_invitations
        WHERE token_hash = $1
        LIMIT 1
        "#,
    )
    .bind(&token_hash_value)
    .fetch_optional(&mut *tx)
    .await?;

    let invitation = invitation.ok_or_else(|| {
        AppError::bad_request("invalid_invitation", "Invitation token is invalid.")
    })?;

    let invitation_id: Uuid = invitation.get("id");
    let workspace_id: Uuid = invitation.get("workspace_id");
    let email: String = invitation.get("email");
    let role_string: String = invitation.get("role");
    let status: String = invitation.get("status");
    let expires_at: DateTime<Utc> = invitation.get("expires_at");
    let accepted_at: Option<DateTime<Utc>> = invitation.get("accepted_at");

    let current_email = fetch_current_user_email(db, auth.user_id).await?;
    if normalize_email(&email) != normalize_email(&current_email) {
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

    let existing = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships
        WHERE workspace_id = $1
          AND user_id = $2
          AND status = 'active'
        "#,
    )
    .bind(workspace_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    if existing > 0 {
        return Err(AppError::conflict(
            "member_conflict",
            "You are already an active member of this workspace.",
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, user_id, role, status, source)
        VALUES ($1, $2, $3::workspace_member_role, 'active', 'invitation')
        ON CONFLICT (workspace_id, user_id)
        DO UPDATE SET
          role = EXCLUDED.role,
          status = 'active',
          source = 'invitation',
          updated_at = NOW()
        "#,
    )
    .bind(workspace_id)
    .bind(auth.user_id)
    .bind(&role_string)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_invitations
        SET status = 'accepted',
            accepted_at = $2,
            updated_at = $2
        WHERE id = $1
        "#,
    )
    .bind(invitation_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    insert_audit_event(
        &mut tx,
        AuditEventInput {
            workspace_id: Some(workspace_id),
            actor_principal_id: Some(auth.principal_id),
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
