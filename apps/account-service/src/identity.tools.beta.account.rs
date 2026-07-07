use anyhow::Context;
use chrono::{Duration, Utc};
use nvbes_product_account::cloud_boundary::{CreateWorkspaceCommand, WorkspacePolicyCommand};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use nvbes_account_service::domains::cloud::workspace_port;

pub struct BetaAccount {
    pub principal_id: Uuid,
    pub tenant_id: Uuid,
}

pub async fn load_or_create_beta_account(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
    password_hash: &str,
) -> anyhow::Result<BetaAccount> {
    if let Some(account) = load_beta_account(tx, email).await? {
        return Ok(account);
    }

    create_beta_account(tx, email, password_hash).await
}

pub async fn normalize_beta_account(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    password_hash: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $2,
            email_verified_at = COALESCE(email_verified_at, NOW()),
            status = 'active',
            password_last_changed_at = NOW(),
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .bind(password_hash)
    .execute(&mut **tx)
    .await
    .context("Failed to normalize beta user credentials.")?;

    sqlx::query("UPDATE principals SET status = 'active', updated_at = NOW() WHERE id = $1")
        .bind(principal_id)
        .execute(&mut **tx)
        .await
        .context("Failed to activate beta principal.")?;

    sqlx::query(
        r#"
        INSERT INTO password_history (principal_id, password_hash)
        VALUES ($1, $2)
        "#,
    )
    .bind(principal_id)
    .bind(password_hash)
    .execute(&mut **tx)
    .await
    .context("Failed to record beta password history.")?;

    Ok(())
}

pub async fn ensure_owner_workspace(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
    workspace_name: &str,
) -> anyhow::Result<()> {
    if has_workspace_role(tenant_id, principal_id, &["owner", "admin"]).await? {
        return Ok(());
    }

    let now = Utc::now();
    workspace_port::create_workspace_tx(
        tx,
        &CreateWorkspaceCommand {
            workspace_id: Uuid::new_v4(),
            tenant_id,
            organization_id: None,
            owner_principal_id: principal_id,
            name: workspace_name.to_string(),
            workspace_type: "personal".to_string(),
            plan_code: "solo_pro".to_string(),
            trial_ends_at: Some(now + Duration::days(14)),
            data_region: "eu".to_string(),
            jurisdiction: "gdpr".to_string(),
            owner_role: "owner".to_string(),
            membership_source: "system".to_string(),
            created_at: now,
            policy: WorkspacePolicyCommand {
                member_can_create_share_links: false,
                require_admin_approval_for_member_share: true,
                default_share_link_ttl_days: 7,
                max_share_link_ttl_days: 30,
                required_acr: Some("aal1".to_string()),
                mfa_policy: None,
            },
        },
    )
    .await
    .map_err(|error| anyhow::anyhow!("Failed to create beta owner workspace: {}", error.message))?;

    Ok(())
}

pub(super) async fn has_workspace_role(
    tenant_id: Uuid,
    principal_id: Uuid,
    roles: &[&str],
) -> anyhow::Result<bool> {
    for workspace in workspace_port::list_tenant_workspaces(tenant_id, None, principal_id)
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to inspect beta workspace list through Cloud: {}",
                error.message
            )
        })?
    {
        for member in workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            principal_id,
        )
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to inspect beta workspace memberships through Cloud: {}",
                error.message
            )
        })? {
            if member.principal_id == principal_id
                && member.active
                && roles.iter().any(|role| *role == member.role)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

async fn load_beta_account(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
) -> anyhow::Result<Option<BetaAccount>> {
    let account = sqlx::query(
        r#"
        SELECT u.principal_id, p.tenant_id
        FROM users u
        INNER JOIN principals p ON p.id = u.principal_id
        WHERE lower(u.email) = lower($1)
        LIMIT 1
        "#,
    )
    .bind(email)
    .fetch_optional(&mut **tx)
    .await
    .context("Failed to load beta account.")?;

    Ok(account.map(|row| BetaAccount {
        principal_id: row.get("principal_id"),
        tenant_id: row.get("tenant_id"),
    }))
}

async fn create_beta_account(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
    password_hash: &str,
) -> anyhow::Result<BetaAccount> {
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let username = beta_username(email);
    let tenant_slug = format!("beta-e2e-{}", tenant_id.simple());

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, 'personal', 'Beta E2E Tenant', $2, 'active', 'standard')
        "#,
    )
    .bind(tenant_id)
    .bind(tenant_slug)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta tenant.")?;

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
        VALUES ($1, $2, 'human', 'active', 'Beta E2E Account')
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta principal.")?;

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, firstname, lastname, username,
          password_hash, email_verified_at, status, password_last_changed_at,
          created_at, updated_at
        )
        VALUES (
          $1, $2, 'Beta', 'E2E', $3,
          $4, NOW(), 'active', NOW(),
          NOW(), NOW()
        )
        "#,
    )
    .bind(principal_id)
    .bind(email)
    .bind(username)
    .bind(password_hash)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta user.")?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, status, source)
        VALUES ($1, $2, 'human', 'active', 'system')
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta tenant membership.")?;

    Ok(BetaAccount {
        principal_id,
        tenant_id,
    })
}

fn beta_username(email: &str) -> String {
    let prefix: String = email
        .split('@')
        .next()
        .unwrap_or("beta")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(32)
        .collect();
    let prefix = if prefix.is_empty() { "beta" } else { &prefix };
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}
