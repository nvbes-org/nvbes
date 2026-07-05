use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

pub use nvbes_core::authz::{ResourceContext, WorkspaceAction, WorkspaceRole};
pub use nvbes_tenancy::WorkspacePolicy;
pub type WorkspaceAccess = nvbes_tenancy::WorkspaceAccess<AuthContext>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminScope {
    Tenant,
    Organization(Uuid),
}

pub fn parse_role(role: &str) -> Result<WorkspaceRole, AppError> {
    nvbes_core::authz::parse_role(role).ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            format!("Unsupported workspace role: {role}."),
        )
    })
}

pub fn parse_identity_role(role: &str) -> Result<nvbes_core::authz::IdentityRole, AppError> {
    nvbes_core::authz::parse_identity_role(role).ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            format!("Unsupported identity role: {role}."),
        )
    })
}

pub fn parse_action(action: &str) -> Result<WorkspaceAction, AppError> {
    nvbes_core::authz::parse_action(action).ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            format!("Unsupported workspace action: {action}."),
        )
    })
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkspaceDecision {
    pub allowed: bool,
    pub reason: String,
    pub action: String,
    pub role: Option<WorkspaceRole>,
    pub requires_step_up: bool,
}

pub trait TenantManagementAuth {
    fn user_id(&self) -> Uuid;
    fn tenant_id(&self) -> Option<Uuid>;
    fn email_verified_at(&self) -> Option<DateTime<Utc>>;
}

impl TenantManagementAuth for crate::domains::auth::types::AuthContext {
    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    fn email_verified_at(&self) -> Option<DateTime<Utc>> {
        self.email_verified_at
    }
}

impl TenantManagementAuth for crate::http::middleware::jwt::AuthContext {
    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    fn email_verified_at(&self) -> Option<DateTime<Utc>> {
        self.email_verified_at
    }
}
