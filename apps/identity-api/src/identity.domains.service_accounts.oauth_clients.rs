use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        oauth::{self, service::CreateOAuthClientInput},
        service_accounts::{
            core::get_service_account,
            helpers::{
                default_drive_audience, enforce_target_role_management, ensure_attached_client,
                new_client_secret, now, record_audit_event, required_tenant_id,
                service_account_client_view_from_oauth_result,
            },
            types::{
                CreateServiceAccountOAuthClientInput, CreateServiceAccountOAuthClientResult,
                RotateOAuthClientSecretResult, ServiceAccountView,
            },
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

pub async fn attach_oauth_client(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    client_id: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;

    let mut tx = db.begin().await?;
    let client = sqlx::query(
        r#"
        SELECT
          id,
          tenant_id,
          owner_scope_type::text AS owner_scope_type,
          owner_scope_id
        FROM oauth_clients
        WHERE client_id = $1
          AND revoked_at IS NULL
          AND client_type = 'service'
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    let tenant_id: Option<Uuid> = client.get("tenant_id");
    if tenant_id != Some(required_tenant_id(access)?) {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "The OAuth client does not belong to this workspace tenant.",
        ));
    }
    let owner_scope_type: String = client.get("owner_scope_type");
    let owner_scope_id: Uuid = client.get("owner_scope_id");
    if owner_scope_type != "workspace" || owner_scope_id != access.workspace_id {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "The OAuth client does not belong to this workspace.",
        ));
    }

    let attached_elsewhere = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM service_accounts
          WHERE client_id = $1
            AND principal_id <> $2
        )
        "#,
    )
    .bind(client_id)
    .bind(service_account_id)
    .fetch_one(&mut *tx)
    .await?;

    if attached_elsewhere {
        return Err(AppError::conflict(
            "service_account_client_attached",
            "This OAuth client is already attached to another service account.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET client_id = $2,
            updated_at = NOW()
        WHERE principal_id = $1
          AND workspace_id = $3
        "#,
    )
    .bind(service_account_id)
    .bind(client_id)
    .bind(access.workspace_id)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.attached",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;
    get_service_account(db, access, service_account_id).await
}

pub async fn rotate_oauth_client_secret(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    client_id: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<RotateOAuthClientSecretResult, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;
    ensure_attached_client(&service_account, client_id)?;

    let client_secret = new_client_secret();
    let client_secret_hash = oauth::hash_client_secret(&client_secret)?;
    let rotated_at = now();
    let mut tx = db.begin().await?;

    sqlx::query(
        r#"
        UPDATE oauth_clients
        SET client_secret_hash = $2
        WHERE client_id = $1
        "#,
    )
    .bind(client_id)
    .bind(&client_secret_hash)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET last_rotated_at = $2,
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(service_account_id)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.secret_rotated",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;

    Ok(RotateOAuthClientSecretResult {
        client_id: client_id.to_string(),
        client_secret,
        rotated_at,
    })
}

pub async fn revoke_oauth_client(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    client_id: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let service_account = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &service_account.role)?;
    ensure_attached_client(&service_account, client_id)?;

    let mut tx = db.begin().await?;
    let client_uuid = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM oauth_clients
        WHERE client_id = $1
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    sqlx::query(
        r#"
        UPDATE oauth_clients
        SET revoked_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(client_uuid)
    .execute(&mut *tx)
    .await?;

    let _ = nvbes_redis::refresh_token::revoke_all_client_refresh_tokens(redis, client_uuid)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;
    let _ = nvbes_redis::par::revoke_pushed_authorization_requests_for_client(redis, client_id)
        .await
        .map_err(|err| {
            AppError::internal(
                "pushed_authorization_request_revoke_failed",
                &err.to_string(),
            )
        })?;

    crate::domains::oauth::authorization_codes::revoke_authorization_codes_for_client(
        redis, client_id,
    )
    .await?;
    crate::domains::oauth::device_codes::revoke_device_codes_for_client(redis, client_id).await?;

    sqlx::query(
        r#"
        UPDATE service_accounts
        SET client_id = NULL,
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(service_account_id)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.revoked",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "oauth_client.detached",
        Some(service_account_id),
        serde_json::json!({
            "client_id": client_id,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;
    get_service_account(db, access, service_account_id).await
}
