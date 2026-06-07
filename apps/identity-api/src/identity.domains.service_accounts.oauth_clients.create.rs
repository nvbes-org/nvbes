use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        oauth::{self, service::CreateOAuthClientInput},
        service_accounts::{
            core::get_service_account,
            policy::{default_drive_audience, enforce_target_role_management},
            types::{CreateServiceAccountOAuthClientInput, CreateServiceAccountOAuthClientResult},
            views::service_account_client_view_from_oauth_result,
        },
    },
    http::error::AppError,
};

pub async fn create_service_account_oauth_client(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    input: CreateServiceAccountOAuthClientInput,
) -> Result<CreateServiceAccountOAuthClientResult, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;
    if !service_account.oauth_clients.is_empty() {
        return Err(AppError::conflict(
            "service_account_client_exists",
            "This service account already has an attached OAuth client.",
        ));
    }

    let client = oauth::clients::create_client(
        db,
        &access.auth,
        CreateOAuthClientInput {
            name: input.name,
            redirect_uris: vec!["https://drive.nvbes.invalid/service-account".to_string()],
            allowed_scopes: input.allowed_scopes,
            allowed_audiences: default_drive_audience(input.allowed_audiences),
            allowed_resources: input.allowed_resources,
            required_acr: input.required_acr,
            client_type: Some("service".to_string()),
            owner_scope_type: Some("workspace".to_string()),
            owner_scope_id: Some(access.workspace_id),
            client_assertion_public_key_jwk: input.client_assertion_public_key_jwk,
            client_assertion_required: input.client_assertion_required,
            service_account_name: Some(service_account.name.clone()),
            service_account_description: service_account.description.clone(),
            service_account_principal_id: Some(service_account_id),
            service_account_role: Some(service_account.role.clone()),
        },
    )
    .await?;

    Ok(CreateServiceAccountOAuthClientResult {
        client: service_account_client_view_from_oauth_result(&client),
        client_secret: client.client_secret,
    })
}
