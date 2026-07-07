use sqlx::Row;
use uuid::Uuid;

use crate::domains::{cloud::workspace_port, oauth::logic::OAuthManagementAuth};
use crate::http::error::AppError;

use crate::domains::oauth::service::types::CreateOAuthClientInput;

#[path = "identity.domains.oauth.clients.service_accounts.persistence.rs"]
mod persistence;

pub(super) async fn ensure_service_account_for_client(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    auth: &impl OAuthManagementAuth,
    client_id: &str,
    workspace_id: Uuid,
    input: &CreateOAuthClientInput,
    service_account_role: &str,
) -> Result<Uuid, AppError> {
    let workspace = workspace_port::get_workspace(
        OAuthManagementAuth::tenant_id(auth),
        workspace_id,
        OAuthManagementAuth::user_id(auth),
    )
    .await?;
    let tenant_id = workspace.tenant_id;
    let organization_id = workspace.organization_id;

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

#[expect(
    clippy::too_many_arguments,
    reason = "Service-account creation keeps workspace, auth, and client metadata explicit."
)]
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

    persistence::insert_tenant_membership(tx, tenant_id, principal_id).await?;

    if let Some(organization_id) = organization_id {
        persistence::insert_organization_membership(tx, organization_id, principal_id).await?;
    }

    persistence::insert_service_account(
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
    persistence::insert_workspace_membership(
        tx,
        OAuthManagementAuth::user_id(auth),
        workspace_id,
        principal_id,
        service_account_role,
    )
    .await?;

    Ok(principal_id)
}
