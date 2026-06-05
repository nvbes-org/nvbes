use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::authz::{ResourceContext, WorkspaceAccess, WorkspaceAction},
    http::error::AppError,
};

use super::auth;
use super::db;
use super::observability;
use super::types::*;

pub async fn list_api_keys(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<ApiKeyListResponse, AppError> {
    let api_keys = db::list_api_keys(db, access.workspace_id).await?;
    Ok(ApiKeyListResponse { api_keys })
}

pub async fn revoke_api_key(
    db: &PgPool,
    access: &WorkspaceAccess,
    api_key_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<RevokeApiKeyResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let api_key = db::update_api_key_status(&mut tx, api_key_id, access.workspace_id, "revoked")
        .await?
        .ok_or_else(|| AppError::not_found("api_key_not_found", "API key not found."))?;

    db::insert_audit_event_tx(
        &mut tx,
        AuditEventInsert {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "api_key.revoked",
            target_type: "api_key",
            target_id: Some(api_key_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({ "key_prefix": api_key.key_prefix }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(RevokeApiKeyResponse { api_key })
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
    auth::authenticate(db, redis, headers, method, uri, request_id, required_scope).await
}

pub async fn authorize_access(
    db: &PgPool,
    headers: &axum::http::HeaderMap,
    context: &PublicApiContext,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceAccess, AppError> {
    auth::authorize_access(db, headers, context, action, resource).await
}

pub async fn me(context: &PublicApiContext) -> ApiIdentityResponse {
    ApiIdentityResponse {
        api_key_id: context.api_key_id.unwrap_or_default(),
        workspace_id: context.workspace_id,
        key_prefix: context.key_prefix.clone(),
        scopes: context.scopes.clone(),
        rate_limit: db::rate_limits_for_plan(&context.plan_code),
    }
}

pub async fn list_workspaces(
    db: &PgPool,
    context: &PublicApiContext,
) -> Result<PublicWorkspacesResponse, AppError> {
    let row = db::get_workspace_for_api(db, context.workspace_id).await?;

    Ok(PublicWorkspacesResponse {
        workspaces: vec![PublicWorkspaceView {
            id: row.get("id"),
            name: row.get("name"),
            plan_code: row.get("plan_code"),
            scopes: context.scopes.clone(),
        }],
    })
}

pub async fn log_request(
    db: &PgPool,
    context: &PublicApiContext,
    input: PublicApiLogInput<'_>,
) -> Result<(), AppError> {
    observability::log_request(db, context, input).await
}

pub async fn record_api_audit_event(
    db: &PgPool,
    context: &PublicApiContext,
    input: PublicApiAuditEventInput<'_>,
) -> Result<(), AppError> {
    observability::record_api_audit_event(db, context, input).await
}
