use anyhow::Context;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn ensure_non_admin_workspace(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    member_principal_id: Uuid,
    workspace_name: &str,
) -> anyhow::Result<()> {
    let owner_principal_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let owner_email = format!("beta-e2e-owner+{}@example.com", workspace_id.simple());

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
        VALUES ($1, $2, 'human', 'active', 'Beta E2E Workspace Owner')
        "#,
    )
    .bind(owner_principal_id)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta e2e owner principal.")?;

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, firstname, lastname, username,
          email_verified_at, status, created_at, updated_at
        )
        VALUES ($1, $2, 'Beta', 'E2E Owner', $3, NOW(), 'active', NOW(), NOW())
        "#,
    )
    .bind(owner_principal_id)
    .bind(owner_email)
    .bind(format!("beta_e2e_owner_{}", workspace_id.simple()))
    .execute(&mut **tx)
    .await
    .context("Failed to create beta e2e owner user.")?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, status, source)
        VALUES ($1, $2, 'human', 'active', 'system')
        ON CONFLICT (tenant_id, principal_id) DO UPDATE
        SET status = 'active', updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(owner_principal_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta e2e owner tenant membership.")?;

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, organization_id, workspace_type, name, plan_code,
          trial_ends_at, data_region, jurisdiction, owner_user_id
        )
        VALUES (
          $1, $2, NULL, 'team', $3, 'trial',
          NOW() + INTERVAL '14 days', 'eu', 'gdpr', $4
        )
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(workspace_name)
    .bind(owner_principal_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta e2e non-admin workspace.")?;

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days
        )
        VALUES ($1, FALSE, TRUE, 7, $2)
        "#,
    )
    .bind(workspace_id)
    .bind(7_i32)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta e2e workspace policy.")?;

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source)
        VALUES
          ($1, $2, 'owner', 'active', 'system'),
          ($1, $3, 'member', 'active', 'system')
        "#,
    )
    .bind(workspace_id)
    .bind(owner_principal_id)
    .bind(member_principal_id)
    .execute(&mut **tx)
    .await
    .context("Failed to create beta e2e workspace memberships.")?;

    Ok(())
}
