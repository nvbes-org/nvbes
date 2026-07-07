use uuid::Uuid;

use crate::{domains::auth::types::AuthContext, http::error::AppError};
use nvbes_tenancy::workspace::WorkspaceAccessError;

pub(super) fn validate_workspace_context(
    auth: &AuthContext,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    if auth
        .tenant_id
        .is_some_and(|tenant| tenant_id.is_some_and(|workspace_tenant| workspace_tenant != tenant))
    {
        return Err(workspace_access_error(WorkspaceAccessError::AccessDenied));
    }

    if auth.organization_id.is_some_and(|organization| {
        organization_id.is_some_and(|workspace_org| workspace_org != organization)
    }) {
        return Err(workspace_access_error(WorkspaceAccessError::AccessDenied));
    }

    if auth
        .workspace_id
        .is_some_and(|current_workspace| current_workspace != workspace_id)
    {
        return Err(workspace_access_error(
            WorkspaceAccessError::ContextMismatch,
        ));
    }

    Ok(())
}

pub(super) fn validate_actor_context(
    auth: &AuthContext,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let Some(actor) = auth.actor.as_ref() else {
        return Ok(());
    };

    if actor.tenant_id.is_some_and(|actor_tenant_id| {
        tenant_id.is_some_and(|workspace_tenant| workspace_tenant != actor_tenant_id)
    }) {
        return Err(workspace_access_error(
            WorkspaceAccessError::DelegatedAccessDenied,
        ));
    }

    if actor.organization_id.is_some_and(|actor_org_id| {
        organization_id.is_some_and(|workspace_org| workspace_org != actor_org_id)
    }) {
        return Err(workspace_access_error(
            WorkspaceAccessError::DelegatedAccessDenied,
        ));
    }

    if actor
        .workspace_id
        .is_some_and(|actor_workspace_id| actor_workspace_id != workspace_id)
    {
        return Err(workspace_access_error(
            WorkspaceAccessError::DelegatedContextMismatch,
        ));
    }

    Ok(())
}

pub(super) fn owns_resource(
    created_by: Uuid,
    created_by_principal_id: Uuid,
    actor_user_id: Uuid,
    actor_principal_id: Uuid,
) -> bool {
    created_by == actor_user_id || created_by_principal_id == actor_principal_id
}

fn workspace_access_error(error: WorkspaceAccessError) -> AppError {
    let code = match error {
        WorkspaceAccessError::AccessDenied
        | WorkspaceAccessError::PrincipalRoleMissing
        | WorkspaceAccessError::DelegatedAccessDenied => "workspace_access_denied",
        WorkspaceAccessError::ContextMismatch | WorkspaceAccessError::DelegatedContextMismatch => {
            "workspace_context_mismatch"
        }
    };

    AppError::forbidden(code, error.to_string())
}
