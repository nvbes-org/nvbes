use crate::domains::auth::types::{AuthContext, StepUpInput, StepUpSubject, SwitchWorkspaceInput};
use crate::domains::auth::verification;
use crate::http::error::AppError;
use nvbes_tenancy::workspace::WorkspaceAccessError;
use sqlx::{PgPool, Row};
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
    db: &PgPool,
    auth: &AuthContext,
    workspace_id: Uuid,
) -> Result<WorkspaceSwitchContext, AppError> {
    let workspace = sqlx::query(
        r#"
        SELECT
          w.id,
          w.tenant_id,
          w.organization_id,
          COALESCE(
            w.owner_user_id,
            (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'admin' LIMIT 1),
            '00000000-0000-0000-0000-000000000000'::uuid
          ) AS owner_principal_id,
          w.name,
          w.workspace_type::text AS workspace_type,
          w.data_region::text AS data_region,
          w.trial_ends_at
        FROM workspaces w
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;

    let membership = sqlx::query(
        r#"
        SELECT role::text AS role
        FROM workspace_memberships
        WHERE workspace_id = $1
          AND principal_id = $2
          AND status = 'active'
        "#,
    )
    .bind(workspace_id)
    .bind(auth.user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            WorkspaceAccessError::AccessDenied.to_string(),
        )
    })?;

    let role = membership.try_get::<String, _>("role").map_err(|_| {
        AppError::internal(
            "workspace_membership_invariant",
            "Workspace membership is inconsistent.",
        )
    })?;

    Ok(WorkspaceSwitchContext {
        workspace_id: workspace.get("id"),
        tenant_id: workspace.get("tenant_id"),
        organization_id: workspace.get("organization_id"),
        owner_principal_id: workspace.get("owner_principal_id"),
        name: workspace.get("name"),
        workspace_type: workspace.get("workspace_type"),
        data_region: workspace.get("data_region"),
        role,
        trial_ends_at: workspace.get("trial_ends_at"),
    })
}

pub(super) async fn ensure_workspace_switch_assurance(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    auth_step_up_ttl_minutes: i64,
    auth: &AuthContext,
    workspace: &WorkspaceSwitchContext,
    input: SwitchWorkspaceInput,
) -> Result<bool, AppError> {
    let assurance = resolve_assurance_context(db, redis, auth, workspace).await?;
    if assurance.sufficient {
        return Ok(false);
    }

    let input = require_step_up_input(input)?;
    verification::step_up(db, redis, auth_step_up_ttl_minutes, webauthn, auth, input).await?;

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
