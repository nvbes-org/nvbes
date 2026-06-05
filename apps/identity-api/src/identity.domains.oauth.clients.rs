use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::oauth::logic::OAuthManagementAuth;
use crate::http::error::AppError;
use nvbes_core::authz::parse_role;

use super::service::types::{
    CreateOAuthClientInput, CreateOAuthClientResult, OAuthClientPolicyView, OAuthClientView,
    OAuthClientsResult, RevokeOAuthClientResult,
};
use super::{hash_client_secret, parse_client_type, parse_step_up_level};

/// List OAuth clients for a tenant.
pub async fn list_clients(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
) -> Result<OAuthClientsResult, AppError> {
    let tenant_id = OAuthManagementAuth::tenant_id(auth).ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let (current_scope_type, current_scope_id) = super::logic::resolve_management_scope(auth)?;
    let rows = sqlx::query(
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
          oauth_clients.client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          sa.principal_id AS service_account_principal_id,
          sa.workspace_id AS service_account_workspace_id,
          wm.role::text AS service_account_role
        FROM oauth_clients
        LEFT JOIN service_accounts sa ON sa.client_id = oauth_clients.client_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
         AND wm.status = 'active'
        WHERE oauth_clients.tenant_id = $1
          AND (
            (oauth_clients.owner_scope_type = $2::scope_type AND oauth_clients.owner_scope_id = $3)
            OR (oauth_clients.owner_scope_type = 'tenant' AND oauth_clients.owner_scope_id = $4)
          )
        ORDER BY oauth_clients.created_at DESC
        "#,
    )
    .bind(OAuthManagementAuth::tenant_id(auth))
    .bind(current_scope_type.as_str())
    .bind(current_scope_id)
    .bind(OAuthManagementAuth::tenant_id(auth))
    .fetch_all(db)
    .await?;

    Ok(OAuthClientsResult {
        clients: rows
            .into_iter()
            .map(|row| OAuthClientView {
                id: row.get("id"),
                client_id: row.get("client_id"),
                name: row.get("name"),
                redirect_uris: row.get("redirect_uris"),
                created_at: row.get("created_at"),
                tenant_id: row.get("tenant_id"),
                owner_scope_type: row.get("owner_scope_type"),
                owner_scope_id: row.get("owner_scope_id"),
                client_type: row.get("client_type"),
                client_assertion_required: row.get("client_assertion_required"),
                client_assertion_public_key_configured: row
                    .get("client_assertion_public_key_configured"),
                service_account_principal_id: row.get("service_account_principal_id"),
                service_account_workspace_id: row.get("service_account_workspace_id"),
                service_account_role: row.get("service_account_role"),
            })
            .collect(),
    })
}

/// Create a new OAuth client.
pub async fn create_client(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    input: CreateOAuthClientInput,
) -> Result<CreateOAuthClientResult, AppError> {
    let tenant_id = OAuthManagementAuth::tenant_id(auth).ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let (owner_scope_type, owner_scope_id) = super::logic::resolve_owner_scope(
        auth,
        input.owner_scope_type.as_deref(),
        input.owner_scope_id,
    )?;
    let client_type = parse_client_type(input.client_type.as_deref().unwrap_or("confidential"))?;
    let client_id = format!("gxoc_{}", Uuid::new_v4().simple());
    let client_secret = format!(
        "gxo_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let client_secret_hash = hash_client_secret(&client_secret)?;
    let required_acr = input.required_acr.as_deref().unwrap_or("aal1");
    let required_acr = parse_step_up_level(required_acr)?;

    if input.redirect_uris.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one redirect URI is required.",
        ));
    }

    if input.allowed_scopes.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one allowed scope is required.",
        ));
    }
    let client_assertion_required = input.client_assertion_required.unwrap_or(false);
    if client_assertion_required && input.client_assertion_public_key_jwk.is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "A client_assertion_public_key_jwk is required when private_key_jwt is mandatory.",
        ));
    }
    if let Some(ref jwk) = input.client_assertion_public_key_jwk {
        if !crate::domains::oauth::client_assertion::is_supported_client_assertion_public_jwk(jwk) {
            return Err(AppError::bad_request(
                "validation_failed",
                "client_assertion_public_key_jwk must be a supported RSA or P-256 public JWK.",
            ));
        }
    }
    let client_assertion_public_key_configured = input.client_assertion_public_key_jwk.is_some();
    let requested_service_account_role = input
        .service_account_role
        .as_deref()
        .unwrap_or("member")
        .trim()
        .to_lowercase();
    if parse_role(&requested_service_account_role).is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Unsupported service account workspace role.",
        ));
    }
    if requested_service_account_role == "owner" {
        let owner_access = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM workspace_memberships
              WHERE workspace_id = $1
                AND principal_id = $2
                AND status = 'active'
                AND role = 'owner'
            )
            "#,
        )
        .bind(owner_scope_id)
        .bind(OAuthManagementAuth::user_id(auth))
        .fetch_one(db)
        .await?;

        if !owner_access {
            return Err(AppError::forbidden(
                "service_account_owner_role_forbidden",
                "Only workspace owners can create owner-level service accounts.",
            ));
        }
    }

    let mut tx = db.begin().await?;
    let client_row = sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          client_id,
          client_secret_hash,
          name,
          redirect_uris,
          tenant_id,
          owner_scope_type,
          owner_scope_id,
          client_type,
          client_assertion_public_key_jwk,
          client_assertion_required
        )
        VALUES ($1, $2, $3, $4, $5, $6::scope_type, $7, $8::client_type, $9, $10)
        RETURNING id, created_at
        "#,
    )
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(&input.name)
    .bind(&input.redirect_uris)
    .bind(OAuthManagementAuth::tenant_id(auth))
    .bind(owner_scope_type.as_str())
    .bind(owner_scope_id)
    .bind(client_type.as_str())
    .bind(input.client_assertion_public_key_jwk.clone())
    .bind(client_assertion_required)
    .fetch_one(&mut *tx)
    .await?;
    let client_uuid: Uuid = client_row.get("id");
    let client_created_at: chrono::DateTime<chrono::Utc> = client_row.get("created_at");
    let service_account_principal_id = if client_type == "service" {
        let workspace_id = require_workspace_scope(&owner_scope_type, owner_scope_id)?;
        Some(
            ensure_service_account_for_client(
                &mut tx,
                auth,
                &client_id,
                workspace_id,
                &input,
                &requested_service_account_role,
            )
            .await?,
        )
    } else {
        None
    };

    let policy = sqlx::query(
        r#"
        INSERT INTO oauth_client_policies (
          client_id,
          scope_type,
          scope_id,
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          required_acr,
          status
        )
        VALUES ($1, $2::scope_type, $3, $4, $5, $6, $7::step_up_level, 'active')
        RETURNING id, scope_type::text AS scope_type, scope_id, allowed_scopes, allowed_audiences, allowed_resources, required_acr::text AS required_acr, status::text AS status, created_at
        "#,
    )
    .bind(client_uuid)
    .bind(owner_scope_type.as_str())
    .bind(owner_scope_id)
    .bind(&input.allowed_scopes)
    .bind(&input.allowed_audiences)
    .bind(&input.allowed_resources)
    .bind(required_acr.as_str())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let is_service_client = client_type == "service";

    Ok(CreateOAuthClientResult {
        client: OAuthClientView {
            id: client_uuid,
            client_id,
            name: input.name,
            redirect_uris: input.redirect_uris,
            created_at: client_created_at,
            tenant_id: OAuthManagementAuth::tenant_id(auth),
            owner_scope_type,
            owner_scope_id,
            client_type: client_type.clone(),
            client_assertion_required,
            client_assertion_public_key_configured,
            service_account_principal_id,
            service_account_workspace_id: is_service_client.then_some(owner_scope_id),
            service_account_role: is_service_client
                .then_some(requested_service_account_role.clone()),
        },
        client_secret,
        policy: OAuthClientPolicyView {
            id: policy.get("id"),
            client_id: client_uuid,
            scope_type: policy.get("scope_type"),
            scope_id: policy.get("scope_id"),
            allowed_scopes: policy.get("allowed_scopes"),
            allowed_audiences: policy.get("allowed_audiences"),
            allowed_resources: policy.get("allowed_resources"),
            required_acr: policy.get("required_acr"),
            status: policy.get("status"),
            created_at: policy.get("created_at"),
        },
    })
}

/// Revoke an OAuth client.
pub async fn revoke_client(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    client_id_str: &str,
) -> Result<RevokeOAuthClientResult, AppError> {
    let tenant_id = OAuthManagementAuth::tenant_id(auth).ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let mut tx = db.begin().await?;

    let client = sqlx::query(
        r#"
        SELECT id, client_id
        FROM oauth_clients
        WHERE client_id = $1
          AND tenant_id = $2
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id_str)
    .bind(OAuthManagementAuth::tenant_id(auth))
    .fetch_optional(&mut *tx)
    .await?;

    let client = client.ok_or_else(|| {
        AppError::not_found(
            "client_not_found",
            "OAuth client not found or already revoked.",
        )
    })?;

    let client_uuid: Uuid = client.get("id");

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

    let tokens_revoked =
        nvbes_redis::refresh_token::revoke_all_client_refresh_tokens(redis, client_uuid)
            .await
            .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;
    let par_revoked =
        nvbes_redis::par::revoke_pushed_authorization_requests_for_client(redis, client_id_str)
            .await
            .map_err(|err| {
                AppError::internal(
                    "pushed_authorization_request_revoke_failed",
                    &err.to_string(),
                )
            })?;

    crate::domains::oauth::authorization_codes::revoke_authorization_codes_for_client(
        redis,
        client_id_str,
    )
    .await?;
    crate::domains::oauth::device_codes::revoke_device_codes_for_client(redis, client_id_str)
        .await?;

    tx.commit().await?;

    tracing::warn!(
        actor_user_id = %OAuthManagementAuth::user_id(auth),
        client_id = %client_id_str,
        client_uuid = %client_uuid,
        tokens_revoked = tokens_revoked,
        par_revoked = par_revoked,
        "OAuth client revoked"
    );

    Ok(RevokeOAuthClientResult {
        client_id: client_id_str.to_string(),
        tokens_revoked,
    })
}

fn require_workspace_scope(owner_scope_type: &str, owner_scope_id: Uuid) -> Result<Uuid, AppError> {
    if owner_scope_type != "workspace" {
        return Err(AppError::bad_request(
            "workspace_scope_required",
            "Service OAuth clients must be scoped to a workspace.",
        ));
    }

    Ok(owner_scope_id)
}

async fn ensure_service_account_for_client(
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

        return Ok(principal_id);
    }

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

    if let Some(organization_id) = organization_id {
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
    }

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

    Ok(principal_id)
}
