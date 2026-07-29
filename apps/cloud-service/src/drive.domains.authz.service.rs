use axum::http::HeaderMap;
use uuid::Uuid;

use super::db::{load_workspace_access, record_permission_denied};
use super::types::{ResourceContext, WorkspaceAccess, WorkspaceAction};
use crate::{
    domains::auth::types::{AuthContext, AuthPrincipalKind},
    http::{error::AppError, request::bearer_token},
};
use nvbes_core::authz::{action_requires_step_up, is_allowed};

pub async fn authorize_workspace_action(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    workspace_id: Uuid,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceAccess, AppError> {
    let token = bearer_token(headers)?;
    let auth = crate::domains::auth::authenticate(db, &token, Some(headers)).await?;
    let access = load_workspace_access(db, &auth, workspace_id).await?;

    if auth.principal_kind == AuthPrincipalKind::User && auth.email_verified_at.is_none() {
        metrics::counter!(
            "drive_authz_denied_total",
            &[("reason", "email_not_verified")]
        )
        .increment(1);
        record_permission_denied(db, &access, action, resource, headers).await?;
        return Err(AppError::forbidden(
            "email_not_verified",
            "Verify your email address before using this workspace.",
        ));
    }
    ensure_scope_allowed(&auth, action)?;

    let mut effective_resource = resource;
    if !effective_resource.member_share_links_enabled {
        effective_resource.member_share_links_enabled = access.policy.member_can_create_share_links;
    }

    if is_allowed(access.role, action, effective_resource) {
        if action_requires_step_up(action) {
            if auth.principal_kind == AuthPrincipalKind::ServiceAccount {
                metrics::counter!(
                    "drive_authz_denied_total",
                    &[("reason", "step_up_not_supported_for_machine")]
                )
                .increment(1);
                return Err(AppError::forbidden(
                    "step_up_not_supported_for_machine",
                    "Step-up actions cannot be performed with a machine token.",
                ));
            }
            crate::domains::auth::require_recent_step_up(db, &auth).await?;
        }
        return Ok(access);
    }

    record_permission_denied(db, &access, action, effective_resource, headers).await?;
    metrics::counter!(
        "drive_authz_denied_total",
        &[("reason", "permission_denied")]
    )
    .increment(1);

    Err(AppError::forbidden(
        "permission_denied",
        "You do not have permission to perform this action in this workspace.",
    ))
}

pub fn ensure_email_verified(auth: &AuthContext) -> Result<(), AppError> {
    if auth.principal_kind == AuthPrincipalKind::ServiceAccount || auth.email_verified_at.is_some()
    {
        return Ok(());
    }

    Err(AppError::forbidden(
        "email_not_verified",
        "Verify your email address before using this workspace.",
    ))
}

fn ensure_scope_allowed(auth: &AuthContext, action: WorkspaceAction) -> Result<(), AppError> {
    let required_scope = required_scope_for_action(action);
    if auth.amr.iter().any(|method| method == "m2m") || auth.actor.is_some() {
        let scopes = auth.scope_list();
        if scopes.contains(&required_scope) {
            return Ok(());
        }

        metrics::counter!(
            "drive_authz_denied_total",
            &[("reason", "insufficient_scope")]
        )
        .increment(1);
        return Err(AppError::forbidden(
            "insufficient_scope",
            "The token does not include the required Drive scope.",
        ));
    }

    Ok(())
}

fn required_scope_for_action(action: WorkspaceAction) -> &'static str {
    match action {
        WorkspaceAction::ViewWorkspace => "drive.workspace.read",
        WorkspaceAction::UpdateWorkspaceSettings => "drive.workspace.manage",
        WorkspaceAction::ViewMembers => "drive.workspace.read",
        WorkspaceAction::InviteMember
        | WorkspaceAction::ChangeMemberRole
        | WorkspaceAction::RemoveMember => "drive.workspace.manage",
        WorkspaceAction::ViewFiles | WorkspaceAction::ViewTrash | WorkspaceAction::DownloadFile => {
            "drive.files.read"
        }
        WorkspaceAction::CreateFolder
        | WorkspaceAction::UploadFile
        | WorkspaceAction::RenameObject
        | WorkspaceAction::MoveObject
        | WorkspaceAction::TrashObject
        | WorkspaceAction::RestoreObject => "drive.files.write",
        WorkspaceAction::DeleteObjectPermanently => "drive.files.delete",
        WorkspaceAction::ViewShareLinks => "drive.share_links.read",
        WorkspaceAction::CreateShareLink
        | WorkspaceAction::UpdateShareLink
        | WorkspaceAction::RevokeShareLink => "drive.share_links.write",
        WorkspaceAction::ViewQuota => "drive.quota.read",
        WorkspaceAction::ViewBilling | WorkspaceAction::ManageBilling => "drive.workspace.manage",
        WorkspaceAction::ViewAudit | WorkspaceAction::ExportAudit => "drive.audit.read",
        WorkspaceAction::ManageKeys | WorkspaceAction::ManageAdministration => "drive.admin",
        WorkspaceAction::ExportWorkspaceData | WorkspaceAction::DeleteWorkspace => {
            "drive.workspace.manage"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::auth::types::{AuthContext, AuthPrincipalKind};
    use chrono::Utc;

    fn machine_auth(scope: &str) -> AuthContext {
        AuthContext {
            principal_id: Uuid::new_v4(),
            principal_kind: AuthPrincipalKind::ServiceAccount,
            user_id: Uuid::new_v4(),
            email_verified_at: Some(Utc::now()),
            session_id: Uuid::nil(),
            tenant_id: Some(Uuid::new_v4()),
            organization_id: None,
            workspace_id: Some(Uuid::new_v4()),
            scope: scope.to_string(),
            role: Some("member".to_string()),
            amr: vec!["m2m".to_string()],
            actor: None,
            acr: None,
            auth_time: Some(Utc::now()),
        }
    }

    #[test]
    fn ensure_scope_allowed_accepts_matching_machine_scope() {
        let auth = machine_auth("drive.files.read drive.workspace.read");
        assert!(ensure_scope_allowed(&auth, WorkspaceAction::ViewFiles).is_ok());
    }

    #[test]
    fn ensure_scope_allowed_rejects_missing_machine_scope() {
        let auth = machine_auth("drive.workspace.read");
        let error = ensure_scope_allowed(&auth, WorkspaceAction::UploadFile)
            .expect_err("missing scope should be rejected");

        assert_eq!(error.code, "insufficient_scope");
    }
}
