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

    verification::require_recent_maximum_assurance_step_up(redis, auth).await?;

    ensure_workspace_has_another_owner(auth).await?;

    let mut tx = db.begin().await?;
    soft_delete_personal_account_tx(&mut tx, auth.user_id()).await?;
    sessions_mgmt::revoke_all_user_sessions_tx(&mut tx, auth.user_id()).await?;
    tx.commit().await?;

    crate::domains::oauth::security_events::enqueue_all_sessions_revoked(
        db,
        redis,
        auth.user_id(),
        None,
    )
    .await?;
    nvbes_redis::session::clear_user_sessions(redis, &auth.user_id().to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))?;

    let _ = nvbes_redis::pubsub::publish_user_suspended(redis, &auth.user_id().to_string()).await;

    Ok(DeleteAccountResult { success: true })
}

async fn soft_delete_personal_account_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: uuid::Uuid,
) -> Result<(), AppError> {
    sqlx::query("UPDATE users SET status = 'deleted', updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query("UPDATE principals SET status = 'deleted', updated_at = NOW() WHERE id = $1")
        .bind(principal_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn ensure_workspace_has_another_owner(auth: &AuthContext) -> Result<(), AppError> {
    for workspace in workspace_port::list_workspaces(None, auth.user_id()).await? {
        let members = workspace_port::list_workspace_members(
            Some(workspace.tenant_id),
            workspace.workspace_id,
            auth.user_id(),
        )
        .await?;
        let owns_workspace = members.iter().any(|member| {
            member.principal_id == auth.user_id() && member.active && member.role == "owner"
        });
        if owns_workspace {
            let active_owner_count = members
                .iter()
                .filter(|member| member.active && member.role == "owner")
                .count();
            if active_owner_count <= 1 {
                return Err(AppError::forbidden(
                    "last_workspace_owner_cannot_delete_account",
                    "Transfer workspace ownership before deleting this account.",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "identity.domains.auth.account_deletion.tests.rs"]
mod tests;
