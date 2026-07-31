use super::db::*;
use super::types::*;
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use axum::http::HeaderMap;
use nvbes_core::authz::{action_requires_step_up, is_allowed};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.authz.service.tenant.rs"]
mod tenant;

pub use tenant::{
    ensure_email_verified, ensure_tenant_context, ensure_tenant_management_access,
    resolve_admin_scope,
};

pub async fn authorize_workspace_action(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
    workspace_id: Uuid,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceAccess, AppError> {
    let auth = authenticate_workspace_bearer(db, redis, jwt, headers).await?;
    let access = load_workspace_access(db, redis, &auth, workspace_id).await?;
    let decision = decide_loaded_workspace_action(redis, &auth, &access, action, resource).await?;

    if decision.allowed {
        return Ok(access);
    }

    let effective_resource =
        effective_resource(access.policy.member_can_create_share_links, resource);
    record_permission_denied(db, &access, action, effective_resource, headers).await?;

    Err(workspace_decision_error(&decision))
}

pub async fn decide_workspace_action(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    authentication: DecisionAuthentication,
    workspace_id: Uuid,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceDecision, AppError> {
    ensure_decision_scope(auth, authentication, action)?;
    let access = load_workspace_access(db, redis, auth, workspace_id).await?;
    decide_loaded_workspace_action(redis, auth, &access, action, resource).await
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionAuthentication {
    BrowserSession,
    OAuthBearer,
}

async fn authenticate_workspace_bearer(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
) -> Result<AuthContext, AppError> {
    crate::domains::auth::sessions::authenticate_bearer(db, redis, jwt, headers).await
}

async fn decide_loaded_workspace_action(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    access: &WorkspaceAccess,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceDecision, AppError> {
    let requires_step_up = action_requires_step_up(action);
    let effective_resource =
        effective_resource(access.policy.member_can_create_share_links, resource);

    if auth.email_verified_at.is_none() {
        return Ok(workspace_decision(
            false,
            "email_not_verified",
            action,
            Some(access.role),
            requires_step_up,
        ));
    }

    if !is_allowed(access.role, action, effective_resource) {
        return Ok(workspace_decision(
            false,
            "permission_denied",
            action,
            Some(access.role),
            requires_step_up,
        ));
    }

    let step_up_satisfied = if !requires_step_up {
        true
    } else if matches!(
        action,
        WorkspaceAction::ChangeMemberRole
            | WorkspaceAction::ManageBilling
            | WorkspaceAction::ExportWorkspaceData
            | WorkspaceAction::DeleteWorkspace
    ) {
        crate::domains::auth::verification::require_recent_maximum_assurance_step_up(redis, auth)
            .await
            .is_ok()
    } else {
        crate::domains::auth::verification::require_recent_phishing_resistant_step_up(redis, auth)
            .await
            .is_ok()
    };

    if requires_step_up && !step_up_satisfied {
        return Ok(workspace_decision(
            false,
            "step_up_required",
            action,
            Some(access.role),
            true,
        ));
    }

    Ok(workspace_decision(
        true,
        "allowed",
        action,
        Some(access.role),
        requires_step_up,
    ))
}

fn effective_resource(
    member_share_links_enabled: bool,
    mut resource: ResourceContext,
) -> ResourceContext {
    if !resource.member_share_links_enabled {
        resource.member_share_links_enabled = member_share_links_enabled;
    }
    resource
}

fn workspace_decision(
    allowed: bool,
    reason: &str,
    action: WorkspaceAction,
    role: Option<WorkspaceRole>,
    requires_step_up: bool,
) -> WorkspaceDecision {
    WorkspaceDecision {
        allowed,
        reason: reason.to_string(),
        action: action.as_str().to_string(),
        role,
        requires_step_up,
    }
}

fn workspace_decision_error(decision: &WorkspaceDecision) -> AppError {
    match decision.reason.as_str() {
        "email_not_verified" => AppError::forbidden(
            "email_not_verified",
            "Verify your email address before using this workspace.",
        ),
        "step_up_required" => nvbes_core::auth::step_up_required_error().into(),
        _ => AppError::forbidden(
            "permission_denied",
            "You do not have permission to perform this action in this workspace.",
        ),
    }
}

fn ensure_decision_scope(
    auth: &AuthContext,
    authentication: DecisionAuthentication,
    action: WorkspaceAction,
) -> Result<(), AppError> {
    if authentication == DecisionAuthentication::BrowserSession {
        return Ok(());
    }
    let scopes = auth
        .scope
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>();
    let accepted = match action {
        WorkspaceAction::ViewWorkspace
        | WorkspaceAction::ViewMembers
        | WorkspaceAction::ViewFiles
        | WorkspaceAction::ViewTrash
        | WorkspaceAction::DownloadFile
        | WorkspaceAction::ViewShareLinks
        | WorkspaceAction::ViewQuota
        | WorkspaceAction::ViewBilling
        | WorkspaceAction::ViewAudit => ["drive:read", "drive:write", "drive:admin"].as_slice(),
        WorkspaceAction::CreateFolder
        | WorkspaceAction::UploadFile
        | WorkspaceAction::RenameObject
        | WorkspaceAction::MoveObject
        | WorkspaceAction::TrashObject
        | WorkspaceAction::RestoreObject
        | WorkspaceAction::DeleteObjectPermanently
        | WorkspaceAction::CreateShareLink
        | WorkspaceAction::UpdateShareLink
        | WorkspaceAction::RevokeShareLink => ["drive:write", "drive:admin"].as_slice(),
        WorkspaceAction::UpdateWorkspaceSettings
        | WorkspaceAction::InviteMember
        | WorkspaceAction::ChangeMemberRole
        | WorkspaceAction::RemoveMember
        | WorkspaceAction::ManageBilling
        | WorkspaceAction::ManageKeys
        | WorkspaceAction::ManageAdministration
        | WorkspaceAction::ExportAudit
        | WorkspaceAction::ExportWorkspaceData
        | WorkspaceAction::DeleteWorkspace => ["drive:admin"].as_slice(),
    };
    if accepted.iter().any(|scope| scopes.contains(scope)) {
        return Ok(());
    }
    Err(AppError::forbidden(
        "insufficient_scope",
        "The access token does not include the Drive scope required for this decision.",
    ))
}

#[cfg(test)]
mod decision_scope_tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{AuthContext, DecisionAuthentication, WorkspaceAction, ensure_decision_scope};

    fn auth(scope: &str) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            user_email: "user@example.test".to_string(),
            display_name: "User".to_string(),
            email_verified_at: Some(Utc::now()),
            mfa_enabled: true,
            tenant_id: Some(Uuid::new_v4()),
            organization_id: None,
            session_id: Uuid::new_v4(),
            workspace_id: Some(Uuid::new_v4()),
            workspace_region: None,
            scope: scope.to_string(),
            acr: Some("aal2".to_string()),
            amr: vec!["webauthn".to_string()],
            auth_time: Some(Utc::now()),
            client_id: Some("cloud-web".to_string()),
            cnf_jkt: None,
        }
    }

    #[test]
    fn read_scope_cannot_preview_an_admin_action() {
        assert!(
            ensure_decision_scope(
                &auth("drive:read"),
                DecisionAuthentication::OAuthBearer,
                WorkspaceAction::DeleteWorkspace
            )
            .is_err()
        );
    }

    #[test]
    fn admin_scope_can_preview_read_write_and_admin_actions() {
        let auth = auth("drive:admin");
        for action in [
            WorkspaceAction::ViewFiles,
            WorkspaceAction::UploadFile,
            WorkspaceAction::DeleteWorkspace,
        ] {
            assert!(
                ensure_decision_scope(&auth, DecisionAuthentication::OAuthBearer, action).is_ok()
            );
        }
    }

    #[test]
    fn browser_session_uses_membership_permissions_without_oauth_scopes() {
        assert!(
            ensure_decision_scope(
                &auth("openid profile email offline_access"),
                DecisionAuthentication::BrowserSession,
                WorkspaceAction::DeleteWorkspace
            )
            .is_ok()
        );
    }
}
