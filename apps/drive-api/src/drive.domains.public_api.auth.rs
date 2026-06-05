use chrono::Utc;
use sqlx::{PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

use crate::{
    domains::auth::types::{AuthContext, AuthPrincipalKind},
    domains::authz::{ResourceContext, WorkspaceAccess, WorkspaceAction},
    http::{error::AppError, request::bearer_token},
};
use nvbes_core::authz::{action_requires_step_up, is_allowed};

use super::db;
use super::observability;
use super::types::{DeniedLogInput, PublicApiContext};
use super::{api_key_signatures, http_signatures};
use sha2::{Digest, Sha256};

enum PublicApiCredential {
    Bearer(String),
    HttpSignature(String),
}

pub async fn authenticate(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &axum::http::HeaderMap,
    method: &axum::http::Method,
    uri: &axum::http::Uri,
    request_id: String,
    required_scope: &str,
) -> Result<PublicApiContext, AppError> {
    metrics::counter!("drive_api_key_legacy_requests_total").increment(1);
    let credential = match http_signatures::signature_identity(headers)? {
        Some(identity) => PublicApiCredential::HttpSignature(identity.key_id),
        None => PublicApiCredential::Bearer(bearer_token(headers)?),
    };
    let hash = match &credential {
        PublicApiCredential::Bearer(token) | PublicApiCredential::HttpSignature(token) => {
            key_hash(token)
        }
    };
    let row_opt = db::get_api_key_by_hash(db, &hash).await?;

    if let Some(row) = row_opt {
        let workspace_id: Uuid = row.get("workspace_id");
        let api_key_id: Uuid = row.get("id");
        let status: String = row.get("status");
        let scopes: Vec<String> = row.get("scopes");
        let plan_code: String = row.get("plan_code");
        let created_by_principal_id: Uuid = row.get("created_by_principal_id");
        let deleted_at: Option<chrono::DateTime<Utc>> = row.get("deleted_at");
        let expires_at: Option<chrono::DateTime<Utc>> = row.get("expires_at");
        let public_key: Option<String> = row.get("http_signature_public_key");

        match credential {
            PublicApiCredential::HttpSignature(_) => {
                let public_key = public_key.as_deref().ok_or_else(|| {
                    AppError::unauthorized(
                        "http_signature_not_enabled",
                        "HTTP signatures are not enabled for this API key.",
                    )
                })?;
                http_signatures::verify(headers, method, uri, public_key)?;
            }
            PublicApiCredential::Bearer(_) if public_key.is_some() => {
                return Err(AppError::unauthorized(
                    "http_signature_required",
                    "This API key requires HTTP Message Signatures.",
                ));
            }
            PublicApiCredential::Bearer(token) => {
                let verified = api_key_signatures::verify(headers, method, uri, &token)?;
                consume_nonce(db, workspace_id, api_key_id, verified).await?;
            }
        }

        if status == "revoked" {
            observability::log_denied(
                db,
                DeniedLogInput {
                    workspace_id,
                    api_key_id: Some(api_key_id),
                    actor_principal_id: Some(created_by_principal_id),
                    request_id: &request_id,
                    error_code: "revoked_api_key",
                    ip: crate::http::request::client_ip(headers).as_deref(),
                    user_agent: crate::http::request::user_agent(headers).as_deref(),
                    scopes_used: &[required_scope],
                },
            )
            .await?;
            return Err(AppError::unauthorized(
                "revoked_api_key",
                "API key has been revoked.",
            ));
        }
        if status == "expired" || expires_at.is_some_and(|expires_at| expires_at <= Utc::now()) {
            observability::log_denied(
                db,
                DeniedLogInput {
                    workspace_id,
                    api_key_id: Some(api_key_id),
                    actor_principal_id: Some(created_by_principal_id),
                    request_id: &request_id,
                    error_code: "expired_api_key",
                    ip: crate::http::request::client_ip(headers).as_deref(),
                    user_agent: crate::http::request::user_agent(headers).as_deref(),
                    scopes_used: &[required_scope],
                },
            )
            .await?;
            return Err(AppError::unauthorized(
                "expired_api_key",
                "API key has expired.",
            ));
        }
        if deleted_at.is_some() {
            observability::log_denied(
                db,
                DeniedLogInput {
                    workspace_id,
                    api_key_id: Some(api_key_id),
                    actor_principal_id: Some(created_by_principal_id),
                    request_id: &request_id,
                    error_code: "workspace_suspended",
                    ip: crate::http::request::client_ip(headers).as_deref(),
                    user_agent: crate::http::request::user_agent(headers).as_deref(),
                    scopes_used: &[required_scope],
                },
            )
            .await?;
            return Err(AppError::forbidden(
                "workspace_suspended",
                "Workspace is not active.",
            ));
        }
        if !scopes.iter().any(|scope| scope == required_scope) {
            observability::log_denied(
                db,
                DeniedLogInput {
                    workspace_id,
                    api_key_id: Some(api_key_id),
                    actor_principal_id: Some(created_by_principal_id),
                    request_id: &request_id,
                    error_code: "insufficient_scope",
                    ip: crate::http::request::client_ip(headers).as_deref(),
                    user_agent: crate::http::request::user_agent(headers).as_deref(),
                    scopes_used: &[required_scope],
                },
            )
            .await?;
            return Err(AppError::forbidden(
                "insufficient_scope",
                "API key does not include the required scope.",
            ));
        }

        let limits = db::rate_limits_for_plan(&plan_code);
        nvbes_core::limiter::check_rate_limit(
            redis,
            "api_key_rate",
            &format!("minute:{api_key_id}"),
            limits.requests_per_minute,
            Duration::from_secs(60),
        )
        .await?;
        nvbes_core::limiter::check_rate_limit(
            redis,
            "api_key_rate",
            &format!("day:{api_key_id}"),
            limits.requests_per_day,
            Duration::from_secs(86_400),
        )
        .await?;

        db::update_last_used(
            db,
            api_key_id,
            crate::http::request::client_ip(headers).as_deref(),
        )
        .await?;

        Ok(PublicApiContext {
            api_key_id: Some(api_key_id),
            workspace_id,
            created_by: Some(row.get("created_by")),
            created_by_principal_id: row.get("created_by_principal_id"),
            tenant_id: None,
            organization_id: None,
            role: None,
            key_prefix: row.get("key_prefix"),
            scopes,
            plan_code,
            request_id,
            m2m_client_id: None,
        })
    } else if let PublicApiCredential::Bearer(token) = &credential {
        let identity_client = crate::domains::auth::identity::IdentityAuthClient::from_env()?;
        let claims = identity_client
            .introspect_access_token(token, Some(headers))
            .await?;

        if claims.active && claims.principal_type.as_deref() == Some("service_account") {
            let scopes: Vec<String> = claims
                .scope
                .unwrap_or_default()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            if !scope_allows_required(&scopes, required_scope) {
                return Err(AppError::forbidden(
                    "insufficient_scope",
                    "M2M token does not include the required scope.",
                ));
            }

            let role = claims.role.clone().ok_or_else(|| {
                AppError::forbidden(
                    "workspace_role_required",
                    "M2M token does not expose a workspace role.",
                )
            })?;

            let workspace_id = claims.workspace_id.ok_or_else(|| {
                AppError::unauthorized("invalid_token", "M2M token is missing workspace_id")
            })?;

            let principal_id = Uuid::parse_str(&claims.sub.unwrap_or_default()).map_err(|_| {
                AppError::unauthorized("invalid_token", "M2M token has invalid sub")
            })?;

            let w_row = super::db::get_workspace_for_api(db, workspace_id)
                .await
                .map_err(|_| AppError::unauthorized("invalid_workspace", "Workspace not found"))?;

            let plan_code: String = w_row.get("plan_code");

            let m2m_client_id = claims.client_id.unwrap_or_default();
            let limits = db::rate_limits_for_plan(&plan_code);
            nvbes_core::limiter::check_rate_limit(
                redis,
                "m2m_rate",
                &format!("minute:{m2m_client_id}"),
                limits.requests_per_minute,
                Duration::from_secs(60),
            )
            .await?;
            nvbes_core::limiter::check_rate_limit(
                redis,
                "m2m_rate",
                &format!("day:{m2m_client_id}"),
                limits.requests_per_day,
                Duration::from_secs(86_400),
            )
            .await?;

            return Ok(PublicApiContext {
                api_key_id: None,
                workspace_id,
                created_by: None,
                created_by_principal_id: principal_id,
                tenant_id: claims.tenant_id,
                organization_id: claims.organization_id,
                role: Some(role),
                key_prefix: "m2m".to_string(),
                scopes,
                plan_code,
                request_id,
                m2m_client_id: Some(m2m_client_id),
            });
        }

        Err(AppError::unauthorized(
            "invalid_api_key",
            "Invalid API key or M2M token.",
        ))
    } else {
        Err(AppError::unauthorized(
            "invalid_api_key",
            "Invalid API key.",
        ))
    }
}

pub async fn authorize_access(
    db: &PgPool,
    headers: &axum::http::HeaderMap,
    context: &PublicApiContext,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceAccess, AppError> {
    let auth = auth_context(context);
    let access =
        crate::domains::authz::db::load_workspace_access(db, &auth, context.workspace_id).await?;

    let mut effective_resource = resource;
    if !effective_resource.member_share_links_enabled {
        effective_resource.member_share_links_enabled = access.policy.member_can_create_share_links;
    }

    if is_allowed(access.role, action, effective_resource) {
        if action_requires_step_up(action) {
            return Err(AppError::forbidden(
                "step_up_not_supported_for_public_api",
                "Step-up actions cannot be performed with Public API credentials.",
            ));
        }

        return Ok(access);
    }

    crate::domains::authz::db::record_permission_denied(
        db,
        &access,
        action,
        effective_resource,
        headers,
    )
    .await?;
    metrics::counter!(
        "drive_authz_denied_total",
        &[("reason", "permission_denied")]
    )
    .increment(1);

    Err(AppError::forbidden(
        "permission_denied",
        "You do not have permission to perform this action in this workspace.",
    ))
}

fn auth_context(context: &PublicApiContext) -> AuthContext {
    AuthContext {
        principal_id: context.created_by_principal_id,
        principal_kind: if context.m2m_client_id.is_some() {
            AuthPrincipalKind::ServiceAccount
        } else {
            AuthPrincipalKind::User
        },
        user_id: context.created_by.unwrap_or_default(),
        email_verified_at: None,
        session_id: context.api_key_id.unwrap_or_default(),
        tenant_id: context.tenant_id,
        organization_id: context.organization_id,
        workspace_id: Some(context.workspace_id),
        scope: auth_scope(&context.scopes),
        role: context.role.clone(),
        amr: vec![if context.m2m_client_id.is_some() {
            "m2m".to_string()
        } else {
            "api_key".to_string()
        }],
        actor: None,
        acr: Some("aal1".to_string()),
        auth_time: None,
    }
}

fn auth_scope(scopes: &[String]) -> String {
    scopes
        .iter()
        .map(|scope| public_scope_to_drive_scope(scope).unwrap_or(scope.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn scope_allows_required(scopes: &[String], required_scope: &str) -> bool {
    let mapped_scope = public_scope_to_drive_scope(required_scope);
    let legacy_mapped_scope = public_scope_to_legacy_drive_scope(required_scope);

    scopes.iter().any(|scope| {
        scope == required_scope
            || mapped_scope.is_some_and(|mapped| scope == mapped)
            || legacy_mapped_scope.is_some_and(|mapped| scope == mapped)
            || scope == "drive:admin"
            || scope == "drive.admin"
    })
}

fn public_scope_to_drive_scope(scope: &str) -> Option<&'static str> {
    match scope {
        "files:read" => Some("drive.files.read"),
        "files:write" => Some("drive.files.write"),
        "files:delete" => Some("drive.files.delete"),
        "workspaces:read" => Some("drive.workspace.read"),
        "workspaces:write" | "workspaces:manage" => Some("drive.workspace.manage"),
        "share_links:read" => Some("drive.share_links.read"),
        "share_links:write" => Some("drive.share_links.write"),
        "audit:read" | "audit_events:read" => Some("drive.audit.read"),
        "quota:read" | "quotas:read" => Some("drive.quota.read"),
        _ => None,
    }
}

fn public_scope_to_legacy_drive_scope(scope: &str) -> Option<&'static str> {
    match scope {
        "files:read" => Some("drive:file:read"),
        "files:write" => Some("drive:file:write"),
        "files:delete" => Some("drive:file:delete"),
        "workspaces:read" => Some("drive:workspace:read"),
        "workspaces:write" | "workspaces:manage" => Some("drive:workspace:manage"),
        "share_links:read" => Some("drive:share_link:read"),
        "share_links:write" => Some("drive:share_link:write"),
        "audit:read" | "audit_events:read" => Some("drive:audit_event:read"),
        "quota:read" | "quotas:read" => Some("drive:quota:read"),
        _ => None,
    }
}

pub fn key_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("{digest:x}")
}

async fn consume_nonce(
    db: &PgPool,
    workspace_id: Uuid,
    api_key_id: Uuid,
    verified: api_key_signatures::VerifiedApiKeySignature,
) -> Result<(), AppError> {
    if db::consume_api_key_nonce(
        db,
        workspace_id,
        api_key_id,
        &verified.nonce,
        verified.timestamp,
    )
    .await?
    {
        return Ok(());
    }
    Err(AppError::unauthorized(
        "api_key_nonce_replayed",
        "X-Nonce has already been used for this API key.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public_api_context(role: Option<String>, m2m_client_id: Option<String>) -> PublicApiContext {
        PublicApiContext {
            api_key_id: None,
            workspace_id: Uuid::new_v4(),
            created_by: None,
            created_by_principal_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            organization_id: None,
            role,
            key_prefix: "m2m".to_string(),
            scopes: vec!["drive.share_links.write".to_string()],
            plan_code: "team".to_string(),
            request_id: "test-request".to_string(),
            m2m_client_id,
        }
    }

    #[test]
    fn auth_context_preserves_m2m_workspace_role() {
        let context = public_api_context(Some("member".to_string()), Some("client".to_string()));
        let auth = auth_context(&context);

        assert_eq!(auth.role.as_deref(), Some("member"));
        assert_eq!(auth.principal_kind, AuthPrincipalKind::ServiceAccount);
    }

    #[test]
    fn scope_allows_public_scope_alias_and_drive_scope() {
        assert!(scope_allows_required(
            &["drive.share_links.write".to_string()],
            "share_links:write"
        ));
        assert!(scope_allows_required(
            &["share_links:write".to_string()],
            "share_links:write"
        ));
        assert!(scope_allows_required(
            &["drive:share_link:write".to_string()],
            "share_links:write"
        ));
        assert!(!scope_allows_required(
            &["drive.files.read".to_string()],
            "share_links:write"
        ));
    }
}
