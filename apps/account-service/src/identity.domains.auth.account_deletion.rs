use crate::cloud_boundary::workspace_port;
use crate::domains::auth::types::StepUpSubject;
use crate::domains::auth::types::{AuthContext, DeleteAccountResult};
use crate::domains::auth::{check_rate_limit, sessions_mgmt, verification};
use crate::http::error::AppError;

pub async fn delete_account(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<DeleteAccountResult, AppError> {
    check_rate_limit(
        redis,
        "auth_me_delete",
        &format!("user:{}", auth.user_id()),
        2,
        std::time::Duration::from_secs(3600),
    )
    .await?;

    verification::require_recent_step_up(redis, auth, Some(nvbes_core::auth::Aal::Aal2)).await?;

    let owned_workspace_ids = owned_workspace_ids(auth).await.unwrap_or_default();

    let mut tx = db.begin().await?;

    sqlx::query("UPDATE users SET status = 'deleted', updated_at = NOW() WHERE principal_id = $1")
        .bind(auth.user_id())
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE principals SET status = 'deleted', updated_at = NOW() WHERE id = $1")
        .bind(auth.user_id())
        .execute(&mut *tx)
        .await?;

    if let Some(tenant_id) = auth.tenant_id() {
        sqlx::query("UPDATE tenants SET status = 'deleted', updated_at = NOW() WHERE id = $1")
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;
    }

    sessions_mgmt::revoke_all_user_sessions_tx(&mut tx, auth.user_id()).await?;
    tx.commit().await?;

    nvbes_redis::session::clear_user_sessions(redis, &auth.user_id().to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))?;

    let _ = nvbes_redis::pubsub::publish_user_suspended(redis, &auth.user_id().to_string()).await;
    for workspace_id in owned_workspace_ids {
        let _ =
            nvbes_redis::pubsub::publish_workspace_deleted(redis, &workspace_id.to_string()).await;
    }

    Ok(DeleteAccountResult { success: true })
}

async fn owned_workspace_ids(auth: &AuthContext) -> Result<Vec<uuid::Uuid>, AppError> {
    let mut owned_workspace_ids = Vec::new();
    for workspace in workspace_port::list_workspaces(None, auth.user_id()).await? {
        let owns_workspace = workspace_port::list_workspace_members(
            Some(workspace.tenant_id),
            workspace.workspace_id,
            auth.user_id(),
        )
        .await?
        .into_iter()
        .any(|member| {
            member.principal_id == auth.user_id() && member.active && member.role == "owner"
        });
        if owns_workspace {
            owned_workspace_ids.push(workspace.workspace_id);
        }
    }
    Ok(owned_workspace_ids)
}
