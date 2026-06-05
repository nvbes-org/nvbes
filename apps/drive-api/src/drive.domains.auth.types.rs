use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthPrincipalKind {
    User,
    ServiceAccount,
}

#[derive(Debug, Clone)]
pub struct AuthActorContext {
    pub principal_id: Uuid,
    pub principal_kind: AuthPrincipalKind,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub role: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub principal_id: Uuid,
    pub principal_kind: AuthPrincipalKind,
    pub user_id: Uuid,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub session_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub scope: String,
    pub role: Option<String>,
    pub amr: Vec<String>,
    pub actor: Option<AuthActorContext>,
    pub acr: Option<String>,
    pub auth_time: Option<DateTime<Utc>>,
}

impl AuthContext {
    pub fn user_id(&self) -> Result<Uuid, crate::http::error::AppError> {
        if self.principal_kind == AuthPrincipalKind::User {
            Ok(self.user_id)
        } else {
            Err(crate::http::error::AppError::forbidden(
                "user_context_required",
                "This action requires a user principal.",
            ))
        }
    }

    pub fn audit_actor_user_id(&self) -> Option<Uuid> {
        if self.principal_kind == AuthPrincipalKind::User {
            Some(self.user_id)
        } else {
            None
        }
    }

    pub fn audit_actor_principal_id(&self) -> Uuid {
        self.principal_id
    }

    pub fn scope_list(&self) -> Vec<&str> {
        self.scope
            .split_whitespace()
            .filter(|scope| !scope.is_empty())
            .collect()
    }
}

#[derive(Debug, Serialize)]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceView {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub role: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct MeResult {
    pub user: UserView,
    pub workspaces: Vec<WorkspaceView>,
    pub current_session_id: Option<Uuid>,
    pub current_tenant_id: Option<Uuid>,
    pub current_organization_id: Option<Uuid>,
    pub current_workspace_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct LogoutResult {
    pub message: &'static str,
}
