use anyhow::Context;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

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
    let existing_owner_workspace: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT wm.workspace_id
        FROM workspace_memberships wm
        WHERE wm.principal_id = $1
          AND wm.status = 'active'
          AND wm.role IN ('owner', 'admin')
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(&mut **tx)
    .await
    .context("Failed to inspect beta owner workspace memberships.")?;

    if existing_owner_workspace.is_some() {
        return Ok(());
    }

    nvbes_identity_api::domains::billing::db::ensure_plan_seeded(tx)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "Failed to seed billing plans for beta owner workspace: {}",
                err.message
            )
        })?;

    let plan_id: Uuid = sqlx::query_scalar(
        r#"
        SELECT id
        FROM plans
        WHERE code = 'solo_pro'
        LIMIT 1
        "#,
    )
    .fetch_one(&mut **tx)
    .await
    .context("Failed to load solo_pro plan for beta owner workspace.")?;

    let workspace_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, organization_id, name, workspace_type, plan_code,
          trial_ends_at, data_region, jurisdiction, owner_user_id, plan_id,
          created_at, updated_at
        )
        VALUES (
          $1, $2, NULL, $3, 'personal', 'solo_pro',
          NOW() + INTERVAL '14 days', 'eu', 'gdpr', $4, $5,
          NOW(), NOW()
        )
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(workspace_name)
    .bind(principal_id)
    .bind(plan_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta owner workspace.")?;

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days,
          required_acr
        )
        VALUES ($1, FALSE, TRUE, 7, 30, 'aal1')
        "#,
    )
    .bind(workspace_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta owner workspace policy.")?;

    sqlx::query(
        r#"
        INSERT INTO subscriptions (
          workspace_id, plan_id, status, billing_provider,
          current_period_start, current_period_end
        )
        VALUES ($1, $2, 'trialing', 'stripe', NOW(), NOW() + INTERVAL '14 days')
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta owner subscription.")?;

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source)
        VALUES ($1, $2, 'owner', 'active', 'system')
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta owner workspace membership.")?;

    Ok(())
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
