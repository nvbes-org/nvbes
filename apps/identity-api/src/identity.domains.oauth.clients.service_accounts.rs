use sqlx::Row;
use uuid::Uuid;

use crate::domains::oauth::logic::OAuthManagementAuth;
use crate::http::error::AppError;

use crate::domains::oauth::service::types::CreateOAuthClientInput;

pub(super) async fn ensure_service_account_for_client(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    auth: &impl OAuthManagementAuth,
    client_id: &str,
    workspace_id: Uuid,
    input: &CreateOAuthClientInput,
    service_account_role: &str,
) -> Result<Uuid, AppError> {
    let workspace = sqlx::query(
        r#"
        SELECT tenant_id, organization_id
        FROM workspaces
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await?;

    let workspace = workspace.ok_or_else(|| {
        AppError::not_found("workspace_not_found", "The target workspace was not found.")
    })?;
    let tenant_id: Uuid = workspace.get("tenant_id");
    let organization_id: Option<Uuid> = workspace.get("organization_id");

    if let Some(principal_id) = input.service_account_principal_id {
        return attach_existing_service_account(
            tx,
            principal_id,
            tenant_id,
            workspace_id,
            client_id,
        )
        .await;
    }

    create_service_account_for_client(
        tx,
        auth,
        client_id,
        workspace_id,
        tenant_id,
        organization_id,
        input,
        service_account_role,
    )
    .await
}

async fn attach_existing_service_account(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    client_id: &str,
) -> Result<Uuid, AppError> {
    let existing = sqlx::query(
        r#"
        SELECT sa.principal_id, sa.workspace_id, p.status::text AS principal_status
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        WHERE sa.principal_id = $1
          AND sa.tenant_id = $2
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await?;

    let existing = existing.ok_or_else(|| {
        AppError::not_found(
            "service_account_not_found",
            "The requested service account was not found.",
        )
    })?;

    if existing
        .get::<Option<Uuid>, _>("workspace_id")
        .is_some_and(|existing_workspace_id| existing_workspace_id != workspace_id)
    {
        return Err(AppError::forbidden(
            "service_account_workspace_mismatch",
            "The requested service account does not belong to this workspace.",
        ));
    }

    if existing.get::<String, _>("principal_status") != "active" {
        return Err(AppError::forbidden(
            "service_account_inactive",
            "The requested service account is not active.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET client_id = $2,
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .bind(client_id)
    .execute(&mut **tx)
    .await?;

    Ok(principal_id)
}

async fn create_service_account_for_client(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    auth: &impl OAuthManagementAuth,
    client_id: &str,
    workspace_id: Uuid,
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
    input: &CreateOAuthClientInput,
    service_account_role: &str,
) -> Result<Uuid, AppError> {
    let principal_id = Uuid::new_v4();
    let display_name = input
        .service_account_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(input.name.as_str());

    sqlx::query(
        r#"
        INSERT INTO principals (
          id,
          tenant_id,
          principal_kind,
          status,
          display_name
        )
        VALUES ($1, $2, 'service_account', 'active', $3)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(display_name)
    .execute(&mut **tx)
    .await?;

    insert_tenant_membership(tx, tenant_id, principal_id).await?;

    if let Some(organization_id) = organization_id {
        insert_organization_membership(tx, organization_id, principal_id).await?;
    }

    insert_service_account(
        tx,
        auth,
        client_id,
        workspace_id,
        tenant_id,
        principal_id,
        display_name,
        input,
    )
    .await?;
    insert_workspace_membership(tx, workspace_id, principal_id, service_account_role).await?;

    Ok(principal_id)
}

async fn insert_tenant_membership(
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

async fn insert_organization_membership(
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

async fn insert_service_account(
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

async fn insert_workspace_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    principal_id: Uuid,
    service_account_role: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (
          workspace_id,
          principal_id,
          role,
          status,
          source
        )
        VALUES ($1, $2, $3::workspace_member_role, 'active', 'system')
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(service_account_role)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
