use crate::cloud_boundary::workspace_port;
use crate::domains::auth::types::{AuthContext, StepUpInput, StepUpSubject, SwitchWorkspaceInput};
use crate::domains::auth::verification;
use crate::http::error::AppError;
use nvbes_core::config::AppConfig;
use nvbes_tenancy::workspace::WorkspaceAccessError;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) struct WorkspaceSwitchContext {
    pub workspace_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub owner_principal_id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: String,
    pub role: String,
    pub trial_ends_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub(super) async fn resolve_workspace_switch_context(
    _db: &PgPool,
    auth: &AuthContext,
    workspace_id: Uuid,
) -> Result<WorkspaceSwitchContext, AppError> {
    let workspace =
        workspace_port::get_workspace(auth.tenant_id, workspace_id, auth.user_id).await?;
    let role = workspace_port::list_workspace_members(
        Some(workspace.tenant_id),
        workspace_id,
        auth.user_id,
    )
    .await?
    .into_iter()
    .find(|member| member.principal_id == auth.user_id && member.active)
    .map(|member| member.role)
    .ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            WorkspaceAccessError::AccessDenied.to_string(),
        )
    })?;

    Ok(WorkspaceSwitchContext {
        workspace_id: workspace.workspace_id,
        tenant_id: Some(workspace.tenant_id),
        organization_id: workspace.organization_id,
        owner_principal_id: workspace.owner_principal_id.unwrap_or_default(),
        name: workspace.name,
        workspace_type: workspace.workspace_type,
        data_region: workspace.data_region.unwrap_or_default(),
        role,
        trial_ends_at: workspace.trial_ends_at,
    })
}

pub(super) async fn ensure_workspace_switch_assurance(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    config: &AppConfig,
    auth: &AuthContext,
    workspace: &WorkspaceSwitchContext,
    input: SwitchWorkspaceInput,
) -> Result<bool, AppError> {
    let assurance = resolve_assurance_context(db, redis, auth, workspace).await?;
    if assurance.sufficient {
        return Ok(false);
    }

    let input = require_step_up_input(input)?;
    verification::step_up(db, redis, config, webauthn, auth, input, false).await?;

    let refreshed = resolve_assurance_context(db, redis, auth, workspace).await?;
    if !refreshed.sufficient {
        return Err(nvbes_core::auth::workspace_switch_step_up_required_error().into());
    }

    Ok(true)
}

fn require_step_up_input(input: SwitchWorkspaceInput) -> Result<StepUpInput, AppError> {
    let step_up = input.into_step_up_input();
    if step_up.password.is_none()
        && step_up.totp_code.is_none()
        && step_up.webauthn_response.is_none()
        && step_up.recovery_code.is_none()
    {
        return Err(nvbes_core::auth::workspace_switch_step_up_required_error().into());
    }

    Ok(step_up)
}

async fn resolve_assurance_context(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    workspace: &WorkspaceSwitchContext,
) -> Result<crate::domains::oauth::service::AssuranceContext, AppError> {
    crate::domains::oauth::assurance::resolve_assurance_context(
        db,
        redis,
        auth.user_id(),
        Some(auth.session_id()),
        auth.client_id.as_deref(),
        workspace.tenant_id,
        workspace.organization_id,
        Some(workspace.workspace_id),
    )
    .await
}
