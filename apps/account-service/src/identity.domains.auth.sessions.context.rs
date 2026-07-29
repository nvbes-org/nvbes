use super::sessions_mgmt::fetch_view;
use crate::cloud_boundary::workspace_port;
use crate::domains::auth::sessions::cache::{apply_workspace_context, current_session_ttl};
use crate::domains::auth::types::{
    AuthContext, SwitchWorkspaceInput, SwitchWorkspaceResult, WorkspaceView,
};
use crate::http::error::AppError;
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.auth.sessions.context.workspace_switch.rs"]
mod workspace_switch;

pub async fn first_workspace_context(
    _db: &PgPool,
    principal_id: Uuid,
) -> Result<(Option<Uuid>, Option<Uuid>, Option<String>), AppError> {
    let mut selected = None;
    let mut selected_rank = i32::MAX;
    for workspace in workspace_port::list_workspaces(None, principal_id).await? {
        let rank = workspace_port::list_workspace_members(
            Some(workspace.tenant_id),
            workspace.workspace_id,
            principal_id,
        )
        .await?
        .into_iter()
        .find(|member| member.principal_id == principal_id && member.active)
        .map(|member| workspace_role_rank(&member.role))
        .unwrap_or(i32::MAX);

        if rank < selected_rank {
            selected_rank = rank;
            selected = Some((
                Some(workspace.workspace_id),
                workspace.organization_id,
                workspace.data_region,
            ));
        }
    }

    Ok(selected.unwrap_or((None, None, None)))
}

fn workspace_role_rank(role: &str) -> i32 {
    match role {
        "owner" => 0,
        "admin" => 1,
        "member" => 2,
        _ => 3,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "workspace switching keeps persistence, cache, WebAuthn policy, authenticated subject, and rotation intent explicit"
)]
pub async fn switch_workspace(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    config: &AppConfig,
    auth: &AuthContext,
    workspace_id: Uuid,
    input: SwitchWorkspaceInput,
    rotate_browser_session: bool,
) -> Result<SwitchWorkspaceResult, AppError> {
    let workspace =
        workspace_switch::resolve_workspace_switch_context(db, auth, workspace_id).await?;
    let stepped_up = workspace_switch::ensure_workspace_switch_assurance(
        db, redis, webauthn, config, auth, &workspace, input,
    )
    .await?;

    let mut browser_session_token = None;
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
        if rotate_browser_session {
            let token = crate::domains::auth::sessions::token::issue(auth.session_id);
            session.browser_session_token_hash =
                Some(crate::domains::auth::sessions::token::hash(&token));
            browser_session_token = Some(token);
        }
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
        } else {
            let _ = crate::domains::auth::sessions::db::insert_session_db(db, &session).await;
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
        browser_session_token,
    })
}
