use axum::http::HeaderMap;
use sqlx::{PgPool, Row};
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use crate::http::request::client_ip;

pub struct DeviceApprovalContextInput<'a> {
    pub auth: &'a AuthContext,
    pub client_id: &'a str,
    pub client_uuid: Uuid,
    pub client_tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub scopes: &'a str,
    pub audience: Option<&'a str>,
    pub resource_indicators: &'a [String],
    pub consent_action: Option<&'a str>,
}

pub struct DeviceApprovalContext {
    pub tenant_id: Uuid,
    pub organization_id: Option<Uuid>,
}

pub async fn enforce_device_authorization_rate_limit(
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    client_id: &str,
) -> Result<(), AppError> {
    let ip_key = client_ip(headers).unwrap_or_else(|| "unknown".to_string());
    crate::domains::auth::check_rate_limit(
        redis,
        "oauth_device_authorize",
        &format!("ip:{ip_key}"),
        12,
        StdDuration::from_secs(60),
    )
    .await?;
    crate::domains::auth::check_rate_limit(
        redis,
        "oauth_device_authorize",
        &format!("key:{client_id}"),
        24,
        StdDuration::from_secs(60),
    )
    .await?;
    Ok(())
}

pub async fn enforce_device_action_rate_limit(
    redis: &nvbes_redis::RedisPool,
    action: &str,
    user_id: Uuid,
    user_code: &str,
) -> Result<(), AppError> {
    crate::domains::auth::check_rate_limit(
        redis,
        action,
        &format!("user:{user_id}"),
        12,
        StdDuration::from_secs(60),
    )
    .await?;
    crate::domains::auth::check_rate_limit(
        redis,
        action,
        &format!("code:{user_code}"),
        6,
        StdDuration::from_secs(60),
    )
    .await?;
    Ok(())
}

pub use enforce_device_action_rate_limit as enforce_device_action_rate_limit_db;
pub use enforce_device_authorization_rate_limit as enforce_device_authorization_rate_limit_db;

pub async fn ensure_device_approval_context(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    input: DeviceApprovalContextInput<'_>,
) -> Result<DeviceApprovalContext, AppError> {
    let session = nvbes_redis::session::get_session(redis, &input.auth.session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", &err.to_string()))?;
    let Some(session) = session else {
        return Err(AppError::unauthorized(
            "invalid_session",
            "Your session is no longer valid.",
        ));
    };
    if session.principal_id != input.auth.user_id.to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::unauthorized(
            "invalid_session",
            "Your session is no longer valid.",
        ));
    }

    let workspace = sqlx::query(
        r#"
        SELECT w.tenant_id, w.organization_id
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE wm.workspace_id = $1
          AND wm.principal_id = $2
          AND wm.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.auth.user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_required",
            "You must be an active member of the selected workspace.",
        )
    })?;

    let workspace_tenant_id: Uuid = workspace.get("tenant_id");
    if workspace_tenant_id != input.client_tenant_id {
        return Err(AppError::forbidden(
            "workspace_tenant_mismatch",
            "The selected workspace does not belong to the OAuth client tenant.",
        ));
    }

    let workspace_organization_id: Option<Uuid> = workspace.get("organization_id");
    if input.organization_id.is_some() && input.organization_id != workspace_organization_id {
        return Err(AppError::bad_request(
            "organization_mismatch",
            "The selected organization does not match the workspace.",
        ));
    }

    let policy = crate::domains::oauth::policies_eval::ensure_client_policy(
        db,
        input.client_id,
        Some(workspace_tenant_id),
        workspace_organization_id,
        Some(input.workspace_id),
        input.scopes,
        input.audience,
        input.resource_indicators,
    )
    .await?;

    crate::domains::oauth::consent::ensure_consent(
        db,
        crate::domains::oauth::service::ConsentRequirementInput {
            user_id: input.auth.user_id,
            client_id: input.client_uuid,
            tenant_id: Some(workspace_tenant_id),
            organization_id: workspace_organization_id,
            workspace_id: Some(input.workspace_id),
            scope: policy.normalized_scope,
            audience: input.audience.map(ToOwned::to_owned),
            resource_indicators: input.resource_indicators.to_vec(),
            consent_action: input.consent_action.map(ToOwned::to_owned),
            policy_status: policy.status,
        },
    )
    .await?;

    Ok(DeviceApprovalContext {
        tenant_id: workspace_tenant_id,
        organization_id: workspace_organization_id,
    })
}

pub async fn ensure_device_deny_context(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    _client_id: &str,
) -> Result<(), AppError> {
    let session = nvbes_redis::session::get_session(redis, &auth.session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", &err.to_string()))?;
    let Some(session) = session else {
        return Err(AppError::unauthorized(
            "invalid_session",
            "Your session is no longer valid.",
        ));
    };
    if session.principal_id != auth.user_id.to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::unauthorized(
            "invalid_session",
            "Your session is no longer valid.",
        ));
    }

    Ok(())
}
