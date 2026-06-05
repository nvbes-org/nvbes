use uuid::Uuid;

use crate::{
    domains::auth::types::AuthContext, domains::authz::WorkspaceAccess, http::error::AppError,
};
use nvbes_tenancy::role_as_str;

use super::db::WorkspaceRecord;
use super::lifecycle;
pub use super::types::*;

pub async fn list_workspaces(
    db: &sqlx::PgPool,
    auth: &AuthContext,
) -> Result<WorkspaceListResponse, AppError> {
    let rows = super::db::list_workspaces(db, auth.user_id, auth.tenant_id).await?;

    Ok(WorkspaceListResponse {
        workspaces: rows.into_iter().map(WorkspaceRecord::into_view).collect(),
    })
}

pub async fn create_workspace(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    input: CreateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    lifecycle::create_workspace(db, auth, input, ip, user_agent).await
}

pub async fn get_workspace(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
) -> Result<WorkspaceResponse, AppError> {
    get_workspace_by_id(db, access.workspace_id, role_as_str(access.role)).await
}

pub async fn update_workspace(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: UpdateWorkspaceInput,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    lifecycle::update_workspace(db, access, input, expected_etag, ip, user_agent).await
}

pub(crate) async fn get_workspace_by_id(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    role: &str,
) -> Result<WorkspaceResponse, AppError> {
    let row = super::db::fetch_workspace_by_id(db, workspace_id, role).await?;

    Ok(WorkspaceResponse {
        workspace: row.into_view(),
    })
}
