use std::collections::HashSet;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domains::{authz::AdminScope, cloud::workspace_port},
    http::error::AppError,
};

pub async fn ensure_workspaces_belong(
    _tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_ids: &[Uuid],
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<(), AppError> {
    let requested = workspace_ids.iter().copied().collect::<HashSet<_>>();
    if requested.len() != workspace_ids.len() {
        return invalid_workspace_scope();
    }

    let organization_id = match scope {
        AdminScope::Tenant => None,
        AdminScope::Organization(organization_id) => Some(organization_id),
    };
    let available =
        workspace_port::list_tenant_workspaces(tenant_id, organization_id, actor_principal_id)
            .await?
            .into_iter()
            .map(|workspace| workspace.workspace_id)
            .collect::<HashSet<_>>();

    if requested
        .iter()
        .all(|workspace_id| available.contains(workspace_id))
    {
        Ok(())
    } else {
        invalid_workspace_scope()
    }
}

fn invalid_workspace_scope() -> Result<(), AppError> {
    Err(AppError::bad_request(
        "invalid_workspace_scope",
        "All workspace IDs must belong to the active tenant/organization scope.",
    ))
}
