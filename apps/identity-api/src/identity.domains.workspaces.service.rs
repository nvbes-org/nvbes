use sqlx::PgPool;

use crate::{
    domains::auth::types::AuthContext, domains::authz::WorkspaceAccess, http::error::AppError,
};

pub use super::types::{
    CreateWorkspaceInput, UpdateWorkspaceInput, WorkspaceListResponse, WorkspaceResponse,
};
use super::{core, db, settings};

pub async fn list_workspaces(
    db: &PgPool,
    auth: &AuthContext,
) -> Result<WorkspaceListResponse, AppError> {
    core::list_workspaces(db, auth).await
}

pub async fn create_workspace(
    db: &PgPool,
    auth: &AuthContext,
    input: CreateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    core::create_workspace(db, auth, input, ip, user_agent).await
}

pub async fn get_workspace(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<WorkspaceResponse, AppError> {
    db::get_workspace_by_id(
        db,
        access.workspace_id,
        nvbes_tenancy::role_as_str(access.role),
    )
    .await
}

pub async fn update_workspace(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: UpdateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    settings::update_workspace(db, access, input, ip, user_agent).await
}
