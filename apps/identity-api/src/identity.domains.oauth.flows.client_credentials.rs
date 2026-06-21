use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::domains::auth::jwt::JwtService;
use crate::http::error::AppError;

use super::{ClientAuthentication, TokenView};

#[path = "identity.domains.oauth.flows.client_credentials.audit.rs"]
mod audit;

pub async fn client_credentials_grant(
    db: &PgPool,
    jwt: &JwtService,
    client_auth: ClientAuthentication,
    scope: Option<&str>,
    audience: Option<&str>,
) -> Result<TokenView, AppError> {
    let audience = audience
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("nvbes-drive-api");
    if audience != "nvbes-drive-api" {
        return Err(AppError::bad_request(
            "invalid_audience",
            "Drive machine tokens must target the nvbes-drive-api audience.",
        ));
    }
    let client = sqlx::query(
        r#"
        SELECT
            oauth_clients.id,
            oauth_clients.client_id,
            oauth_clients.client_secret_hash,
            oauth_clients.client_assertion_required,
            oauth_clients.client_type::text AS client_type,
            oauth_clients.owner_scope_type::text AS owner_scope_type,
            oauth_clients.owner_scope_id,
            sa.principal_id AS service_account_principal_id,
            sa.workspace_id AS service_account_workspace_id,
            w.tenant_id,
            w.organization_id,
            w.data_region::text AS workspace_region,
            wm.role::text AS service_account_role,
            p.status::text AS principal_status
        FROM oauth_clients
        LEFT JOIN service_accounts sa ON sa.client_id = oauth_clients.client_id
        LEFT JOIN workspaces w ON w.id = sa.workspace_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
         AND wm.status = 'active'
        LEFT JOIN principals p ON p.id = sa.principal_id
        WHERE oauth_clients.client_id = $1
          AND oauth_clients.revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(&client_auth.client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::unauthorized(
            "invalid_client",
            "The client is not registered or has been revoked.",
        )
    })?;

    let client_uuid: Uuid = client.get("id");
    let client_secret_hash: String = client.get("client_secret_hash");
    let client_assertion_required: bool = client.get("client_assertion_required");
    let client_type: String = client.get("client_type");
    let owner_scope_type: String = client.get("owner_scope_type");
    let owner_scope_id: Uuid = client.get("owner_scope_id");
    let service_account_principal_id: Option<Uuid> = client.get("service_account_principal_id");
    let service_account_workspace_id: Option<Uuid> = client.get("service_account_workspace_id");
    let tenant_id: Option<Uuid> = client.get("tenant_id");
    let organization_id: Option<Uuid> = client.get("organization_id");
    let workspace_region: Option<String> = client.get("workspace_region");
    let service_account_role: Option<String> = client.get("service_account_role");
    let principal_status: Option<String> = client.get("principal_status");

    if crate::domains::oauth::validation::is_public_client_type(&client_type) {
        return Err(AppError::unauthorized(
            "unauthorized_client",
            "Public OAuth clients cannot use client_credentials.",
        ));
    }

    if client_assertion_required && !client_auth.client_assertion_verified {
        return Err(AppError::unauthorized(
            "invalid_client",
            "private_key_jwt client authentication is required.",
        ));
    }

    if !client_auth.client_assertion_verified {
        let client_secret = client_auth.client_secret.as_deref().ok_or_else(|| {
            AppError::unauthorized(
                "invalid_client",
                "Client authentication is required for client_credentials grant.",
            )
        })?;
        crate::domains::oauth::verify_client_secret_with_overlap(
            db,
            &client_auth.client_id,
            client_secret,
            &client_secret_hash,
        )
        .await?;
    }

    let scope_str = scope.unwrap_or("");
    if !scope_str.is_empty() {
        let policy = sqlx::query(
            r#"
            SELECT
                allowed_scopes,
                allowed_audiences,
                status::text AS status
            FROM oauth_client_policies
            WHERE client_id = $1
              AND scope_type = $2::scope_type
              AND scope_id = $3
            LIMIT 1
            "#,
        )
        .bind(client_uuid)
        .bind(&owner_scope_type)
        .bind(owner_scope_id)
        .fetch_optional(db)
        .await?;

        if let Some(policy) = policy {
            let status: String = policy.get("status");
            if status != "active" {
                return Err(AppError::forbidden(
                    "client_not_allowed",
                    "The OAuth client policy is not active.",
                ));
            }

            let allowed_scopes: Vec<String> = policy.get("allowed_scopes");
            let allowed_scopes = crate::domains::oauth::normalize_scopes(allowed_scopes);
            let allowed_audiences = crate::domains::oauth::normalize_resources(
                policy.get::<Vec<String>, _>("allowed_audiences"),
            );
            let requested_scopes = crate::domains::oauth::normalize_scopes(
                scope_str
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect(),
            );
            if !requested_scopes
                .iter()
                .all(|s| allowed_scopes.iter().any(|a| a == s))
            {
                return Err(AppError::forbidden(
                    "client_scope_not_allowed",
                    "The requested scope is not approved for this OAuth client.",
                ));
            }
            if !allowed_audiences.is_empty()
                && !allowed_audiences.iter().any(|allowed| allowed == audience)
            {
                return Err(AppError::forbidden(
                    "invalid_scope",
                    "The requested audience is not approved for this OAuth client.",
                ));
            }
        }
    }

    let service_account_principal_id = service_account_principal_id.ok_or_else(|| {
        AppError::forbidden(
            "service_account_required",
            "This OAuth client is not attached to a workspace service account.",
        )
    })?;
    let workspace_id = service_account_workspace_id.ok_or_else(|| {
        AppError::forbidden(
            "workspace_scope_required",
            "This OAuth client is not attached to a workspace service account.",
        )
    })?;
    let tenant_id = tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "This OAuth client is not attached to a tenant workspace.",
        )
    })?;
    if principal_status.as_deref() != Some("active") || service_account_role.is_none() {
        return Err(AppError::forbidden(
            "service_account_inactive",
            "The OAuth client service account is inactive or unassigned.",
        ));
    }
    if owner_scope_type != "workspace" || owner_scope_id != workspace_id {
        return Err(AppError::forbidden(
            "workspace_scope_required",
            "Machine-to-machine Drive clients must be scoped to a workspace.",
        ));
    }

    let access_token = jwt.generate_m2m_access_token(
        &client_auth.client_id,
        service_account_principal_id,
        tenant_id,
        organization_id,
        workspace_id,
        workspace_region,
        scope_str,
        Some(audience),
    )?;
    let claims = jwt.decode_token(&access_token, "access")?;
    audit::record_machine_token_issued(
        db,
        audit::MachineTokenAuditInput {
            client_uuid,
            client_id: &client_auth.client_id,
            service_account_principal_id,
            tenant_id,
            workspace_id,
            jti: &claims.jti,
            scope: scope_str,
            audience,
        },
    )
    .await?;
    metrics::counter!("identity_oauth_client_credentials_total").increment(1);

    Ok(TokenView {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt.access_token_expiry.num_seconds(),
        refresh_token: None,
        scope: scope_str.to_string(),
        authorization_details: Vec::new(),
        issued_token_type: None,
    })
}

#[cfg(test)]
#[path = "identity.domains.oauth.flows.client_credentials.tests.rs"]
mod tests;
