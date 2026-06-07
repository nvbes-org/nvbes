use sqlx::PgPool;

use crate::{
    domains::auth::types::{AuthContext, AuthPrincipalKind},
    domains::authz::{ResourceContext, WorkspaceAccess, WorkspaceAction},
    domains::public_api::errors::PublicApiErrorKind,
    http::error::AppError,
};
use nvbes_core::authz::{action_requires_step_up, is_allowed};

use super::super::types::PublicApiContext;
use super::scopes::auth_scope;

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
            return Err(PublicApiErrorKind::StepUpNotSupported.app_error());
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

    Err(PublicApiErrorKind::PermissionDenied.app_error())
}

pub(super) fn auth_context(context: &PublicApiContext) -> AuthContext {
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
