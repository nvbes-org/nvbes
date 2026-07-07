use anyhow::Context;
use chrono::{Duration, Utc};
use nvbes_product_account::cloud_boundary::{
    CreateWorkspaceCommand, UpsertWorkspaceMembershipCommand, WorkspacePolicyCommand,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use nvbes_account_service::domains::cloud::workspace_port;

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

    let now = Utc::now();
    workspace_port::create_workspace_tx(
        tx,
        &CreateWorkspaceCommand {
            workspace_id,
            tenant_id,
            organization_id: None,
            owner_principal_id,
            name: workspace_name.to_string(),
            workspace_type: "team".to_string(),
            plan_code: "trial".to_string(),
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
                max_share_link_ttl_days: 7,
                required_acr: Some("aal1".to_string()),
                mfa_policy: None,
            },
        },
    )
    .await
    .map_err(|error| {
        anyhow::anyhow!(
            "Failed to create beta e2e non-admin workspace through Cloud: {}",
            error.message
        )
    })?;
    workspace_port::upsert_workspace_membership_tx(
        tx,
        &UpsertWorkspaceMembershipCommand {
            actor_principal_id: owner_principal_id,
            workspace_id,
            principal_id: member_principal_id,
            role: "member".to_string(),
            status: "active".to_string(),
            source: "system".to_string(),
        },
    )
    .await
    .map_err(|error| {
        anyhow::anyhow!(
            "Failed to create beta e2e workspace member through Cloud: {}",
            error.message
        )
    })?;

    Ok(())
}
