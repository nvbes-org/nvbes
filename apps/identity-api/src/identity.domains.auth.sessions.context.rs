use super::sessions_mgmt::fetch_view;
use crate::domains::auth::sessions::cache::{apply_workspace_context, current_session_ttl};
use crate::domains::auth::types::{
    AuthContext, StepUpInput, SwitchWorkspaceInput, SwitchWorkspaceResult, WorkspaceView,
};
use crate::domains::auth::verification;
use crate::http::error::AppError;
use sqlx::{PgPool, Row};
use uuid::Uuid;

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
    let policy_row = sqlx::query(
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
          w.trial_ends_at,
          wp.required_acr::text AS required_acr,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days
        FROM workspaces w
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
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
    .await?;
    let membership = membership.ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            "You do not have access to this workspace.",
        )
    })?;
    let role = membership.try_get::<String, _>("role").map_err(|_| {
        AppError::internal(
            "workspace_membership_invariant",
            "Workspace membership is inconsistent.",
        )
    })?;

    let tenant_id: Option<Uuid> = policy_row.get("tenant_id");
    let organization_id: Option<Uuid> = policy_row.get("organization_id");
    let mut stepped_up = false;
    let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
        db,
        redis,
        auth.user_id,
        Some(auth.session_id),
        auth.client_id.as_deref(),
        tenant_id,
        organization_id,
        Some(workspace_id),
    )
    .await?;

    if !assurance.sufficient {
        if input.password.is_none()
            && input.totp_code.is_none()
            && input.webauthn_response.is_none()
            && input.recovery_code.is_none()
        {
            return Err(AppError::unauthorized(
                "step_up_required",
                "Please verify again before switching workspaces.",
            ));
        }

        verification::step_up(
            db,
            redis,
            auth_step_up_ttl_minutes,
            webauthn,
            auth,
            StepUpInput {
                password: input.password,
                totp_code: input.totp_code,
                webauthn_response: input.webauthn_response,
                webauthn_challenge_id: input.webauthn_challenge_id,
                recovery_code: input.recovery_code,
            },
        )
        .await?;
        stepped_up = true;

        let refreshed = crate::domains::oauth::assurance::resolve_assurance_context(
            db,
            redis,
            auth.user_id,
            Some(auth.session_id),
            auth.client_id.as_deref(),
            tenant_id,
            organization_id,
            Some(workspace_id),
        )
        .await?;
        if !refreshed.sufficient {
            return Err(AppError::unauthorized(
                "step_up_required",
                "Please verify again before switching workspaces.",
            ));
        }
    }

    if let Ok(Some(mut session)) =
        nvbes_redis::session::get_session(redis, &auth.session_id.to_string()).await
    {
        apply_workspace_context(
            &mut session,
            tenant_id,
            organization_id,
            Some(workspace_id),
            Some(policy_row.get("data_region")),
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
            id: policy_row.get("id"),
            owner_principal_id: policy_row.get("owner_principal_id"),
            name: policy_row.get("name"),
            workspace_type: policy_row.get("workspace_type"),
            data_region: policy_row.get("data_region"),
            role,
            trial_ends_at: policy_row.get("trial_ends_at"),
        },
        session: fetch_view(redis, auth.session_id, false).await?,
        stepped_up,
    })
}
