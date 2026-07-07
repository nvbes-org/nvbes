use crate::{
    domains::{auth::types::AuthContext, authz::WorkspaceAccess, cloud::workspace_port},
    http::error::AppError,
};
use sqlx::PgPool;

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
    _db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<WorkspaceResponse, AppError> {
    let workspace =
        workspace_port::get_workspace(access.tenant_id, access.workspace_id, access.auth.user_id)
            .await?;
    Ok(db::workspace_response_from_cloud(
        workspace,
        nvbes_tenancy::role_as_str(access.role),
    ))
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
