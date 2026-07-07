use nvbes_core::authz::WorkspaceRole;
use uuid::Uuid;

use crate::{
    domains::{
        authz::{WorkspaceAccess, parse_role},
        service_accounts::types::ServiceAccountView,
    },
    http::error::AppError,
};

pub const DRIVE_AUDIENCE: &str = "nvbes-cloud-service";

pub fn require_name(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 120 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Service account name is invalid.",
        ));
    }
    Ok(value.to_string())
}

pub fn normalize_requested_role(
    actor_role: WorkspaceRole,
    requested_role: Option<&str>,
) -> Result<WorkspaceRole, AppError> {
    let target_role = parse_role(requested_role.unwrap_or("member"))?;
    enforce_target_role_assignment(actor_role, target_role)?;
    Ok(target_role)
}

pub fn enforce_target_role_management(
    actor_role: WorkspaceRole,
    current_target_role: &str,
) -> Result<(), AppError> {
    let target_role = parse_role(current_target_role)?;
    enforce_target_role_assignment(actor_role, target_role)
}

fn enforce_target_role_assignment(
    actor_role: WorkspaceRole,
    target_role: WorkspaceRole,
) -> Result<(), AppError> {
    if actor_role == WorkspaceRole::Owner {
        return Ok(());
    }

    if actor_role == WorkspaceRole::Admin
        && matches!(target_role, WorkspaceRole::Member | WorkspaceRole::Viewer)
    {
        return Ok(());
    }

    Err(AppError::forbidden(
        "permission_denied",
        "This workspace role cannot manage the requested service account level.",
    ))
}

pub fn required_tenant_id(access: &WorkspaceAccess) -> Result<Uuid, AppError> {
    access.tenant_id.ok_or_else(|| {
        AppError::internal(
            "tenant_context_missing",
            "A tenant context is required for service account management.",
        )
    })
}

pub fn default_drive_audience(mut allowed_audiences: Vec<String>) -> Vec<String> {
    if !allowed_audiences
        .iter()
        .any(|audience| audience == DRIVE_AUDIENCE)
    {
        allowed_audiences.push(DRIVE_AUDIENCE.to_string());
    }
    allowed_audiences
}

pub fn ensure_attached_client(
    service_account: &ServiceAccountView,
    client_id: &str,
) -> Result<(), AppError> {
    if service_account
        .oauth_clients
        .iter()
        .any(|client| client.client_id == client_id)
    {
        return Ok(());
    }

    Err(AppError::not_found(
        "client_not_found",
        "The OAuth client is not attached to this service account.",
    ))
}
