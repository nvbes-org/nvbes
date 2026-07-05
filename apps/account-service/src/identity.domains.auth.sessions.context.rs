use super::sessions_mgmt::fetch_view;
use crate::domains::auth::sessions::cache::{apply_workspace_context, current_session_ttl};
use crate::domains::auth::types::{
    AuthContext, SwitchWorkspaceInput, SwitchWorkspaceResult, WorkspaceView,
};
use crate::http::error::AppError;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[path = "identity.domains.auth.sessions.context.workspace_switch.rs"]
mod workspace_switch;

pub async fn first_workspace_context(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<(Option<Uuid>, Option<Uuid>, Option<String>), AppError> {
    let row = sqlx::query(
        r#"
        SELECT w.id AS workspace_id, w.tenant_id, w.organization_id, w.data_region::text AS data_region
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE wm.principal_id = $1 AND wm.status = 'active'
        ORDER BY CASE wm.role::text WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END, wm.created_at ASC
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await?;

    Ok(row
        .map(|row| {
            (
                Some(row.get("workspace_id")),
                row.get("organization_id"),
                Some(row.get::<String, _>("data_region")),
            )
        })
        .unwrap_or((None, None, None)))
}

pub async fn switch_workspace(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    auth_step_up_ttl_minutes: i64,
    auth: &AuthContext,
    workspace_id: Uuid,
    input: SwitchWorkspaceInput,
) -> Result<SwitchWorkspaceResult, AppError> {
    let workspace =
        workspace_switch::resolve_workspace_switch_context(db, auth, workspace_id).await?;
    let stepped_up = workspace_switch::ensure_workspace_switch_assurance(
        db,
        redis,
        webauthn,
        auth_step_up_ttl_minutes,
        auth,
        &workspace,
        input,
    )
    .await?;

    if let Ok(Some(mut session)) =
        nvbes_redis::session::get_session(redis, &auth.session_id.to_string()).await
    {
        apply_workspace_context(
            &mut session,
            workspace.tenant_id,
            workspace.organization_id,
            Some(workspace.workspace_id),
            Some(workspace.data_region.clone()),
        );
        let ttl = current_session_ttl(&session);
        if nvbes_redis::session::set_session(redis, &session, ttl)
            .await
            .is_err()
        {
            let _ = nvbes_redis::session::delete_session(
                redis,
                &auth.user_id.to_string(),
                &auth.session_id.to_string(),
            )
            .await;
        }
    }

    Ok(SwitchWorkspaceResult {
        workspace: WorkspaceView {
            id: workspace.workspace_id,
            owner_principal_id: workspace.owner_principal_id,
            name: workspace.name,
            workspace_type: workspace.workspace_type,
            data_region: workspace.data_region,
            role: workspace.role,
            trial_ends_at: workspace.trial_ends_at,
        },
        session: fetch_view(redis, auth.session_id, false).await?,
        stepped_up,
    })
}
