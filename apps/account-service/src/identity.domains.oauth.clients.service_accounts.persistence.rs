use uuid::Uuid;

use crate::domains::oauth::logic::OAuthManagementAuth;
use crate::domains::oauth::service::types::CreateOAuthClientInput;
use crate::http::error::AppError;
use nvbes_product_account::cloud_boundary::UpsertWorkspaceMembershipCommand;

pub(super) async fn insert_tenant_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (
          tenant_id,
          principal_id,
          principal_kind,
          status,
          source
        )
        VALUES ($1, $2, 'service_account', 'active', 'system')
        ON CONFLICT (tenant_id, principal_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub(super) async fn insert_organization_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO organization_memberships (
          organization_id,
          principal_id,
          status,
          source
        )
        VALUES ($1, $2, 'active', 'system')
        ON CONFLICT (organization_id, principal_id) DO NOTHING
        "#,
    )
    .bind(organization_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Service-account persistence keeps ownership and OAuth metadata explicit."
)]
pub(super) async fn insert_service_account(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    auth: &impl OAuthManagementAuth,
    client_id: &str,
    workspace_id: Uuid,
    tenant_id: Uuid,
    principal_id: Uuid,
    display_name: &str,
    input: &CreateOAuthClientInput,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO service_accounts (
          principal_id,
          tenant_id,
          workspace_id,
          created_by_principal_id,
          name,
          description,
          auth_method,
          client_id,
          secret_hash,
          public_key_jwk,
          last_rotated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'oauth_client_credentials', $7, NULL, NULL, NOW())
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(OAuthManagementAuth::user_id(auth))
    .bind(display_name)
    .bind(input.service_account_description.as_deref())
    .bind(client_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub(super) async fn insert_workspace_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor_principal_id: Uuid,
    workspace_id: Uuid,
    principal_id: Uuid,
    service_account_role: &str,
) -> Result<(), AppError> {
    crate::cloud_boundary::workspace_port::upsert_workspace_membership_tx(
        tx,
        &UpsertWorkspaceMembershipCommand {
            actor_principal_id,
            workspace_id,
            principal_id,
            role: service_account_role.to_string(),
            status: "active".to_string(),
            source: "system".to_string(),
        },
    )
    .await?;

    Ok(())
}
