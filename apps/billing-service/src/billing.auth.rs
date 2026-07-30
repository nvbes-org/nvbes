use axum::http::{HeaderMap, header};
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
    pub auth_time: Option<i64>,
    pub authentication_event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingWorkspacePermission {
    Read,
    Manage,
}

pub async fn authorize_billing_workspace(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    workspace_id: Uuid,
    permission: BillingWorkspacePermission,
) -> Result<BillingAuthContext, AppError> {
    let token = bearer_token(headers)?;
    let identity = introspect_identity_token(headers, &token).await?;
    if identity
        .workspace_id
        .is_some_and(|claim| claim != workspace_id)
    {
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
            "Billing changes require a recent phishing-resistant passkey or security-key authentication.",
        ));
    }
    if permission == BillingWorkspacePermission::Manage {
        tracing::info!(
            security_event = "privileged_authentication_enforced",
            privileged_surface = "billing_and_secret_management",
            principal_id = %identity.principal_id,
            workspace_id = %workspace_id,
            authentication_event_id = identity.authentication_event_id.as_deref().unwrap_or_default(),
            acr = identity.acr.as_deref().unwrap_or_default(),
            amr = %identity.amr.join(","),
            auth_time = identity.auth_time.unwrap_or_default(),
            "accepted privileged Billing request"
        );
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
    let identity = crate::identity_grpc::introspect(headers, token).await?;
    identity_auth_context(identity)
}

fn identity_auth_context(
    identity: crate::grpc::pb::nvbes::identity::internal::v1::IntrospectAccessTokenResponse,
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
    let tenant_id = optional_uuid(identity.tenant_id, "tenant_id")?;
    let workspace_id = optional_uuid(identity.workspace_id, "workspace_id")?;

    Ok(BillingAuthContext {
        principal_id,
        tenant_id,
        workspace_id,
        scope: identity.scope,
        acr: identity.acr,
        amr: identity.amr,
        auth_time: identity.auth_time,
        authentication_event_id: identity.sid,
    })
}

fn optional_uuid(value: Option<String>, field: &str) -> Result<Option<Uuid>, AppError> {
    value
        .map(|value| {
            Uuid::parse_str(&value).map_err(|_| {
                AppError::internal(
                    "identity_grpc_response_invalid",
                    format!("Identity gRPC response contains an invalid {field}."),
                )
            })
        })
        .transpose()
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
    auth.authentication_event_id
        .as_deref()
        .is_some_and(|event_id| !event_id.trim().is_empty())
        && nvbes_core::auth::has_recent_phishing_resistant_authentication(
            auth.acr.as_deref(),
            &auth.amr,
            auth.auth_time,
            chrono::Utc::now().timestamp(),
            nvbes_core::auth::PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        )
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
    fn billing_management_requires_recent_phishing_resistant_step_up() {
        let now = chrono::Utc::now().timestamp();
        let base = BillingAuthContext {
            principal_id: Uuid::nil(),
            tenant_id: None,
            workspace_id: None,
            scope: None,
            acr: None,
            amr: Vec::new(),
            auth_time: Some(now),
            authentication_event_id: Some("billing-authn-event".to_string()),
        };
        assert!(!has_step_up(&base));
        assert!(!has_step_up(&BillingAuthContext {
            acr: Some("aal2".to_string()),
            ..base.clone()
        }));
        assert!(!has_step_up(&BillingAuthContext {
            acr: Some("aal2".to_string()),
            amr: vec!["otp".to_string()],
            ..base.clone()
        }));
        assert!(has_step_up(&BillingAuthContext {
            acr: Some("aal2".to_string()),
            amr: vec!["webauthn".to_string()],
            ..base.clone()
        }));
        assert!(!has_step_up(&BillingAuthContext {
            acr: Some("aal2".to_string()),
            amr: vec!["webauthn".to_string()],
            authentication_event_id: None,
            ..base.clone()
        }));
        assert!(!has_step_up(&BillingAuthContext {
            acr: Some("aal2".to_string()),
            auth_time: Some(now - 16 * 60),
            ..base
        }));
    }
}
