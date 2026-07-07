use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domains::{
    cloud::workspace_port,
    developer::types::{
        CreateDeveloperAppRequest, CreateDeveloperAppResponse, DeveloperAppView,
        DeveloperAppsResponse, UpdateDeveloperRedirectsRequest,
    },
    oauth::{
        logic::OAuthManagementAuth,
        service::types::{CreateOAuthClientInput, OAuthClientView, RevokeOAuthClientResult},
    },
};
use crate::http::error::AppError;

pub async fn list_apps(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
) -> Result<DeveloperAppsResponse, AppError> {
    let result = crate::domains::oauth::clients::list_clients(db, auth).await?;

    Ok(DeveloperAppsResponse {
        apps: result
            .clients
            .into_iter()
            .map(DeveloperAppView::from)
            .collect(),
    })
}

pub async fn get_app(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<DeveloperAppView, AppError> {
    let client = fetch_client_by_client_id(db, tenant_id, client_id).await?;

    Ok(DeveloperAppView::from(client))
}

pub async fn create_app(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    request: CreateDeveloperAppRequest,
) -> Result<CreateDeveloperAppResponse, AppError> {
    let tenant_id = OAuthManagementAuth::tenant_id(auth).ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required for developer app management.",
        )
    })?;
    let result = crate::domains::oauth::clients::create_client(
        db,
        auth,
        CreateOAuthClientInput {
            name: request.name,
            redirect_uris: request.redirect_uris,
            allowed_scopes: request.allowed_scopes,
            allowed_audiences: request.allowed_audiences,
            allowed_resources: request.allowed_resources,
            required_acr: Some("aal1".to_string()),
            client_type: Some("confidential".to_string()),
            owner_scope_type: Some("tenant".to_string()),
            owner_scope_id: Some(tenant_id),
            client_assertion_public_key_jwk: None,
            client_assertion_required: Some(false),
            requires_admin_consent: Some(false),
            service_account_name: None,
            service_account_description: None,
            service_account_principal_id: None,
            service_account_role: None,
        },
    )
    .await?;

    Ok(CreateDeveloperAppResponse {
        app: DeveloperAppView::from(result.client),
        client_secret: result.client_secret,
    })
}

pub async fn update_redirects(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
    request: UpdateDeveloperRedirectsRequest,
) -> Result<DeveloperAppView, AppError> {
    if request.redirect_uris.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one redirect URI is required.",
        ));
    }

    let row = sqlx::query(
        r#"
        UPDATE oauth_clients
        SET redirect_uris = $3,
            updated_at = NOW()
        WHERE client_id = $1
          AND tenant_id = $2
          AND revoked_at IS NULL
        RETURNING id, client_id, name, redirect_uris, created_at, tenant_id,
          owner_scope_type::text AS owner_scope_type, owner_scope_id, client_type::text AS client_type,
          client_assertion_required, client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          NULL::uuid AS service_account_principal_id, NULL::uuid AS service_account_workspace_id,
          NULL::text AS service_account_role
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .bind(&request.redirect_uris)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found."))?;

    Ok(DeveloperAppView::from(oauth_client_view_from_row(
        &row, None,
    )))
}

pub async fn revoke_app(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    client_id: &str,
) -> Result<RevokeOAuthClientResult, AppError> {
    crate::domains::oauth::clients::revoke_client(db, redis, auth, client_id).await
}

async fn fetch_client_by_client_id(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<OAuthClientView, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          oauth_clients.id,
          oauth_clients.client_id,
          oauth_clients.name,
          oauth_clients.redirect_uris,
          oauth_clients.created_at,
          oauth_clients.tenant_id,
          oauth_clients.owner_scope_type::text AS owner_scope_type,
          oauth_clients.owner_scope_id,
          oauth_clients.client_type::text AS client_type,
          oauth_clients.client_assertion_required,
          oauth_clients.requires_admin_consent,
          oauth_clients.client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          sa.principal_id AS service_account_principal_id,
          sa.workspace_id AS service_account_workspace_id
        FROM oauth_clients
        LEFT JOIN service_accounts sa ON sa.client_id = oauth_clients.client_id
        WHERE oauth_clients.client_id = $1
          AND oauth_clients.tenant_id = $2
          AND oauth_clients.revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found."))?;

    let service_account_principal_id: Option<Uuid> = row.get("service_account_principal_id");
    let service_account_workspace_id: Option<Uuid> = row.get("service_account_workspace_id");
    let service_account_role = match (service_account_principal_id, service_account_workspace_id) {
        (Some(principal_id), Some(workspace_id)) => {
            workspace_port::list_workspace_members(Some(tenant_id), workspace_id, principal_id)
                .await?
                .into_iter()
                .find(|member| member.principal_id == principal_id && member.active)
                .map(|member| member.role)
        }
        _ => None,
    };

    Ok(oauth_client_view_from_row(&row, service_account_role))
}

fn oauth_client_view_from_row(
    row: &PgRow,
    service_account_role: Option<String>,
) -> OAuthClientView {
    OAuthClientView {
        id: row.get("id"),
        client_id: row.get("client_id"),
        name: row.get("name"),
        redirect_uris: row.get("redirect_uris"),
        created_at: row.get("created_at"),
        last_used_at: None,
        tenant_id: row.get("tenant_id"),
        owner_scope_type: row.get("owner_scope_type"),
        owner_scope_id: row.get("owner_scope_id"),
        client_type: row.get("client_type"),
        client_assertion_required: row.get("client_assertion_required"),
        requires_admin_consent: row.get("requires_admin_consent"),
        client_assertion_public_key_configured: row.get("client_assertion_public_key_configured"),
        service_account_principal_id: row.get("service_account_principal_id"),
        service_account_workspace_id: row.get("service_account_workspace_id"),
        service_account_role,
    }
}

impl From<OAuthClientView> for DeveloperAppView {
    fn from(client: OAuthClientView) -> Self {
        Self {
            id: client.id,
            client_id: client.client_id,
            name: client.name,
            redirect_uris: client.redirect_uris,
            client_type: client.client_type,
            created_at: client.created_at,
            last_used_at: None::<DateTime<Utc>>,
        }
    }
}
