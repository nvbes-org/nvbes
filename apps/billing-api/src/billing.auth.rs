use axum::http::{HeaderMap, header};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct BillingAuthContext {
    pub principal_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub scope: Option<String>,
    pub acr: Option<String>,
    pub amr: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingWorkspacePermission {
    Read,
    Manage,
}

#[derive(Debug, Deserialize)]
struct IdentityIntrospectionResponse {
    active: bool,
    scope: Option<String>,
    principal_type: Option<String>,
    sub: Option<String>,
    role: Option<String>,
    tenant_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    acr: Option<String>,
    #[serde(default)]
    amr: Vec<String>,
    network_valid: Option<bool>,
}

pub async fn authorize_billing_workspace(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    workspace_id: Uuid,
    permission: BillingWorkspacePermission,
) -> Result<BillingAuthContext, AppError> {
    let token = bearer_token(headers)?;
    let identity = introspect_identity_token(headers, &token).await?;
    if identity.workspace_id.is_some_and(|claim| claim != workspace_id) {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "Token workspace context does not match the requested workspace.",
        ));
    }

    let role = workspace_role(db, workspace_id, identity.principal_id).await?;
    if !role_allows(role.as_str(), permission) {
        return Err(AppError::forbidden(
            "billing_workspace_access_denied",
            "This principal cannot access billing for this workspace.",
        ));
    }
    if permission == BillingWorkspacePermission::Manage && !has_step_up(&identity) {
        return Err(AppError::forbidden(
            "step_up_required",
            "Billing changes require a recent step-up authentication.",
        ));
    }

    Ok(identity)
}

fn bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::unauthorized("missing_authorization", "Missing bearer token."))
}

async fn introspect_identity_token(
    headers: &HeaderMap,
    token: &str,
) -> Result<BillingAuthContext, AppError> {
    let client_id = std::env::var("NVBES_IDENTITY_CLIENT_ID").map_err(|_| {
        AppError::internal(
            "identity_client_id_missing",
            "NVBES_IDENTITY_CLIENT_ID is required to authorize Billing requests.",
        )
    })?;
    let client_secret = std::env::var("NVBES_IDENTITY_CLIENT_SECRET").map_err(|_| {
        AppError::internal(
            "identity_client_secret_missing",
            "NVBES_IDENTITY_CLIENT_SECRET is required to authorize Billing requests.",
        )
    })?;
    let base_url = std::env::var("NVBES_IDENTITY_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut outgoing = HeaderMap::new();
    nvbes_observability::propagate_headers_trace_context(headers, &mut outgoing);
    let response = reqwest::Client::new()
        .post(format!("{}/oauth/introspect", base_url.trim_end_matches('/')))
        .headers(outgoing)
        .basic_auth(client_id, Some(client_secret))
        .json(&serde_json::json!({
            "token": token,
            "token_type_hint": "access_token",
        }))
        .send()
        .await
        .map_err(|error| AppError::internal("identity_introspection_failed", error.to_string()))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(AppError::unauthorized(
            "invalid_token",
            "The access token is invalid or expired.",
        ));
    }
    if !response.status().is_success() {
        return Err(AppError::internal(
            "identity_introspection_failed",
            format!("Identity introspection failed with status {}.", response.status()),
        ));
    }

    let identity = response
        .json::<IdentityIntrospectionResponse>()
        .await
        .map_err(|error| {
            AppError::internal("identity_introspection_invalid_response", error.to_string())
        })?;
    identity_auth_context(identity)
}

fn identity_auth_context(
    identity: IdentityIntrospectionResponse,
) -> Result<BillingAuthContext, AppError> {
    if !identity.active {
        return Err(AppError::unauthorized(
            "invalid_token",
            "The access token is inactive.",
        ));
    }
    if identity.network_valid == Some(false) {
        return Err(AppError::forbidden(
            "network_restriction_violated",
            "Access denied: current location does not match network restrictions.",
        ));
    }
    if identity.principal_type.as_deref() != Some("user") {
        return Err(AppError::forbidden(
            "user_context_required",
            "Billing workspace actions require a user principal.",
        ));
    }
    let principal_id = identity
        .sub
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| AppError::unauthorized("invalid_token", "Invalid Identity subject."))?;

    Ok(BillingAuthContext {
        principal_id,
        tenant_id: identity.tenant_id,
        workspace_id: identity.workspace_id,
        scope: identity.scope,
        acr: identity.acr,
        amr: identity.amr,
    })
}

async fn workspace_role(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    principal_id: Uuid,
) -> Result<String, AppError> {
    let row = sqlx::query(
        r#"
        SELECT wm.role::text AS role
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE wm.workspace_id = $1
          AND wm.principal_id = $2
          AND wm.status = 'active'
          AND w.deleted_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?;

    row.map(|row| row.get("role")).ok_or_else(|| {
        AppError::forbidden(
            "billing_workspace_access_denied",
            "This principal cannot access billing for this workspace.",
        )
    })
}

fn role_allows(role: &str, permission: BillingWorkspacePermission) -> bool {
    match permission {
        BillingWorkspacePermission::Read => matches!(role, "owner" | "admin" | "member" | "viewer"),
        BillingWorkspacePermission::Manage => matches!(role, "owner" | "admin"),
    }
}

fn has_step_up(auth: &BillingAuthContext) -> bool {
    auth.acr.as_deref() == Some("aal2")
        || auth
            .amr
            .iter()
            .any(|method| matches!(method.as_str(), "otp" | "webauthn" | "passkey"))
}

#[cfg(test)]
mod tests {
    use super::{BillingAuthContext, BillingWorkspacePermission, has_step_up, role_allows};
    use uuid::Uuid;

    #[test]
    fn billing_role_policy_separates_read_and_manage() {
        assert!(role_allows("viewer", BillingWorkspacePermission::Read));
        assert!(!role_allows("viewer", BillingWorkspacePermission::Manage));
        assert!(role_allows("admin", BillingWorkspacePermission::Manage));
    }

    #[test]
    fn step_up_accepts_aal2_or_strong_amr() {
        let base = BillingAuthContext {
            principal_id: Uuid::nil(),
            tenant_id: None,
            workspace_id: None,
            scope: None,
            acr: None,
            amr: Vec::new(),
        };
        assert!(!has_step_up(&base));
        assert!(has_step_up(&BillingAuthContext {
            acr: Some("aal2".to_string()),
            ..base.clone()
        }));
        assert!(has_step_up(&BillingAuthContext {
            amr: vec!["webauthn".to_string()],
            ..base
        }));
    }
}
