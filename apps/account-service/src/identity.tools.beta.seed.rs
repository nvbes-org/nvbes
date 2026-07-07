use super::{account, workspace};
use anyhow::Context;
use sqlx::PgPool;

pub struct PrepareBetaE2eAccountInput {
    pub email: String,
    pub password: String,
    pub workspace_name: String,
}

pub async fn prepare_beta_e2e_account(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    input: PrepareBetaE2eAccountInput,
) -> anyhow::Result<()> {
    let password_hash = nvbes_core::auth::hash_password(&input.password)
        .map_err(|err| anyhow::anyhow!(err.message))?;
    let gate_workspace_name = format!("{} Non-Admin Gate", input.workspace_name);

    let mut tx = pool
        .begin()
        .await
        .context("Failed to begin beta e2e seed transaction.")?;

    let account = account::load_or_create_beta_account(&mut tx, &input.email, &password_hash)
        .await
        .context("Failed to ensure beta account.")?;
    let principal_id = account.principal_id;
    let tenant_id = account.tenant_id;

    account::normalize_beta_account(&mut tx, principal_id, &password_hash).await?;
    account::ensure_owner_workspace(&mut tx, tenant_id, principal_id, &input.workspace_name)
        .await?;

    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'revoked', last_used_at = COALESCE(last_used_at, NOW())
        WHERE principal_id = $1
          AND status = 'active'
        "#,
    )
    .bind(principal_id)
    .execute(&mut *tx)
    .await
    .context("Failed to disable beta MFA factors.")?;

    sqlx::query(
        r#"
        DELETE FROM risk_events
        WHERE principal_id = $1
          AND event_type IN (
            'login_failed',
            'password_reset_requested',
            'password_reset_completed'
          )
        "#,
    )
    .bind(principal_id)
    .execute(&mut *tx)
    .await
    .context("Failed to clear beta risk events.")?;

    if !account::has_workspace_role(tenant_id, principal_id, &["member", "viewer"]).await? {
        workspace::ensure_non_admin_workspace(
            &mut tx,
            tenant_id,
            principal_id,
            &gate_workspace_name,
        )
        .await?;
    }

    tx.commit()
        .await
        .context("Failed to commit beta e2e seed transaction.")?;

    reset_beta_rate_limits(redis, &input.email).await?;

    Ok(())
}

async fn reset_beta_rate_limits(redis: &nvbes_redis::RedisPool, email: &str) -> anyhow::Result<()> {
    for (action, key) in [
        ("auth_forgot_password", format!("key:{email}")),
        ("auth_forgot_password", "ip:unknown".to_string()),
        ("auth_forgot_password", "ip:127.0.0.1".to_string()),
        ("auth_reset_password", "ip:unknown".to_string()),
        ("auth_reset_password", "ip:127.0.0.1".to_string()),
        ("auth_login", format!("key:{email}")),
    ] {
        nvbes_redis::rate_limit::reset_rate_limit(redis, "default", action, &key)
            .await
            .with_context(|| format!("Failed to reset rate limit {action}:{key}."))?;
    }

    Ok(())
}
