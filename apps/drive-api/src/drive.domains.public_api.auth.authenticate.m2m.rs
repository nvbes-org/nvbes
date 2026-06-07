use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

use super::super::super::{db, errors::PublicApiErrorKind, types::PublicApiContext};
use super::super::scopes::scope_allows_required;

pub(super) async fn authenticate_m2m(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &axum::http::HeaderMap,
    token: &str,
    request_id: String,
    required_scope: &str,
) -> Result<PublicApiContext, AppError> {
    let identity_client = crate::domains::auth::identity::IdentityAuthClient::from_env()?;
    let claims = identity_client
        .introspect_access_token(token, Some(headers))
        .await?;

    if !(claims.active && claims.principal_type.as_deref() == Some("service_account")) {
        return Err(PublicApiErrorKind::InvalidApiKey.app_error());
    }

    let scopes: Vec<String> = claims
        .scope
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();

    if !scope_allows_required(&scopes, required_scope) {
        return Err(PublicApiErrorKind::InsufficientM2mScope.app_error());
    }

    let role = claims
        .role
        .clone()
        .ok_or_else(|| PublicApiErrorKind::WorkspaceRoleRequired.app_error())?;

    let workspace_id = claims
        .workspace_id
        .ok_or_else(|| PublicApiErrorKind::InvalidTokenMissingWorkspaceId.app_error())?;

    let principal_id = Uuid::parse_str(&claims.sub.unwrap_or_default())
        .map_err(|_| PublicApiErrorKind::InvalidTokenInvalidSub.app_error())?;

    let workspace = db::get_workspace_for_api(db, workspace_id)
        .await
        .map_err(|_| PublicApiErrorKind::InvalidWorkspace.app_error())?;

    let plan_code: String = workspace.get("plan_code");
    let m2m_client_id = claims.client_id.unwrap_or_default();
    super::enforce_plan_rate_limit(redis, "m2m_rate", &m2m_client_id, &plan_code).await?;

    Ok(PublicApiContext {
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
    })
}
