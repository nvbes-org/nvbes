use super::types::{PublicApiAuditEventInput, PublicApiContext, PublicApiLogInput};
use crate::{
    domains::authz::{ResourceContext, WorkspaceAccess, WorkspaceAction},
    http::error::AppError,
};
use axum::http::{HeaderMap, Method, Uri};
use nvbes_observability::request_id_from_headers;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn authenticate(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    required_scope: &str,
) -> Result<PublicApiContext, AppError> {
    crate::domains::public_api::authenticate(
        db,
        redis,
        headers,
        method,
        uri,
        request_id_from_headers(headers),
        required_scope,
    )
    .await
}

pub async fn scoped_access(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    workspace_id: Uuid,
    required_scope: &str,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<(PublicApiContext, WorkspaceAccess), AppError> {
    let ctx = authenticate(db, redis, headers, method, uri, required_scope).await?;
    if ctx.workspace_id != workspace_id {
        return Err(AppError::forbidden(
            "workspace_suspended",
            "API key is not scoped to this workspace.",
        ));
    }
    let access =
        crate::domains::public_api::authorize_access(db, headers, &ctx, action, resource).await?;
    Ok((ctx, access))
}

pub async fn authorize_scoped_action(
    db: &PgPool,
    headers: &HeaderMap,
    ctx: &PublicApiContext,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> Result<WorkspaceAccess, AppError> {
    crate::domains::public_api::authorize_access(db, headers, ctx, action, resource).await
}

pub async fn scoped_object_access(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    workspace_id: Uuid,
    required_scope: &str,
    object_id: Uuid,
    action: WorkspaceAction,
) -> Result<(PublicApiContext, WorkspaceAccess), AppError> {
    let (ctx, access) = scoped_access(
        db,
        redis,
        headers,
        method,
        uri,
        workspace_id,
        required_scope,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;
    let resource = crate::domains::authz::resource_context_for_object(
        db,
        workspace_id,
        access.auth.user_id,
        access.auth.principal_id,
        object_id,
    )
    .await?;
    authorize_scoped_action(db, headers, &ctx, action, resource).await?;
    Ok((ctx, access))
}

pub async fn scoped_share_link_access(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    workspace_id: Uuid,
    required_scope: &str,
    share_link_id: Uuid,
    action: WorkspaceAction,
) -> Result<(PublicApiContext, WorkspaceAccess), AppError> {
    let (ctx, access) = scoped_access(
        db,
        redis,
        headers,
        method,
        uri,
        workspace_id,
        required_scope,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;
    let resource = crate::domains::authz::resource_context_for_share_link(
        db,
        workspace_id,
        access.auth.user_id,
        access.auth.principal_id,
        share_link_id,
    )
    .await?;
    authorize_scoped_action(db, headers, &ctx, action, resource).await?;
    Ok((ctx, access))
}

pub async fn log_ok(
    db: &PgPool,
    headers: &HeaderMap,
    ctx: &PublicApiContext,
    method: &str,
    path: &str,
    scopes: &[&str],
) -> Result<(), AppError> {
    crate::domains::public_api::log_request(
        db,
        ctx,
        PublicApiLogInput {
            method,
            path,
            status_code: 200,
            error_code: None,
            scopes_used: scopes,
            ip: crate::http::request::client_ip(headers).as_deref(),
            user_agent: crate::http::request::user_agent(headers).as_deref(),
        },
    )
    .await
}

pub async fn record_api_event(
    db: &PgPool,
    headers: &HeaderMap,
    ctx: &PublicApiContext,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    crate::domains::public_api::record_api_audit_event(
        db,
        ctx,
        PublicApiAuditEventInput {
            action,
            target_type,
            target_id,
            ip: crate::http::request::client_ip(headers).as_deref(),
            user_agent: crate::http::request::user_agent(headers).as_deref(),
            metadata,
        },
    )
    .await
}
