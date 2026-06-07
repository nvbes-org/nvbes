use super::types::{PublicApiContext, PublicApiLogInput};
use crate::{
    domains::authz::{ResourceContext, WorkspaceAccess, WorkspaceAction},
    domains::public_api::errors::PublicApiErrorKind,
    http::error::AppError,
};
use axum::http::{HeaderMap, Method, Uri};
use sqlx::PgPool;
use uuid::Uuid;

use super::request_meta::PublicApiRequestMeta;

pub struct PublicApiRequestContext {
    pub meta: PublicApiRequestMeta,
    pub ctx: PublicApiContext,
}

pub struct PublicApiAuthorizedRequest {
    pub request: PublicApiRequestContext,
    pub access: WorkspaceAccess,
}

pub async fn authenticate(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    required_scope: &str,
) -> Result<PublicApiRequestContext, AppError> {
    let meta = PublicApiRequestMeta::from_headers(headers);
    let ctx = crate::domains::public_api::authenticate(
        db,
        redis,
        headers,
        method,
        uri,
        meta.request_id_owned(),
        required_scope,
    )
    .await?;
    Ok(PublicApiRequestContext { meta, ctx })
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
) -> Result<PublicApiAuthorizedRequest, AppError> {
    let request = authenticate(db, redis, headers, method, uri, required_scope).await?;
    if request.ctx.workspace_id != workspace_id {
        return Err(PublicApiErrorKind::WorkspaceScopeMismatch.app_error());
    }
    let access =
        crate::domains::public_api::authorize_access(db, headers, &request.ctx, action, resource)
            .await?;
    Ok(PublicApiAuthorizedRequest { request, access })
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

enum ScopedResourceTarget {
    Object(Uuid),
    ShareLink(Uuid),
}

async fn scoped_resource_access(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    workspace_id: Uuid,
    required_scope: &str,
    target: ScopedResourceTarget,
    action: WorkspaceAction,
) -> Result<PublicApiAuthorizedRequest, AppError> {
    let authorized = scoped_access(
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

    let resource = match target {
        ScopedResourceTarget::Object(object_id) => {
            crate::domains::authz::resource_context_for_object(
                db,
                workspace_id,
                authorized.access.auth.user_id,
                authorized.access.auth.principal_id,
                object_id,
            )
            .await?
        }
        ScopedResourceTarget::ShareLink(share_link_id) => {
            crate::domains::authz::resource_context_for_share_link(
                db,
                workspace_id,
                authorized.access.auth.user_id,
                authorized.access.auth.principal_id,
                share_link_id,
            )
            .await?
        }
    };
    authorize_scoped_action(db, headers, &authorized.request.ctx, action, resource).await?;
    Ok(authorized)
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
) -> Result<PublicApiAuthorizedRequest, AppError> {
    scoped_resource_access(
        db,
        redis,
        headers,
        method,
        uri,
        workspace_id,
        required_scope,
        ScopedResourceTarget::Object(object_id),
        action,
    )
    .await
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
) -> Result<PublicApiAuthorizedRequest, AppError> {
    scoped_resource_access(
        db,
        redis,
        headers,
        method,
        uri,
        workspace_id,
        required_scope,
        ScopedResourceTarget::ShareLink(share_link_id),
        action,
    )
    .await
}

pub async fn log_ok(
    db: &PgPool,
    request: &PublicApiRequestContext,
    method: &str,
    path: &str,
    scopes: &[&str],
) -> Result<(), AppError> {
    crate::domains::public_api::log_request(
        db,
        &request.ctx,
        PublicApiLogInput {
            method,
            path,
            status_code: 200,
            error_code: None,
            scopes_used: scopes,
            ip: request.meta.ip(),
            user_agent: request.meta.user_agent(),
        },
    )
    .await
}
