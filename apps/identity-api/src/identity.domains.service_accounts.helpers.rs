use chrono::Utc;
use nvbes_audit::AuditEventInput;
use nvbes_core::authz::WorkspaceRole;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    domains::{
        authz::{WorkspaceAccess, parse_role},
        oauth,
        service_accounts::types::{ServiceAccountClientView, ServiceAccountView},
    },
    http::error::AppError,
};

pub const DRIVE_AUDIENCE: &str = "nvbes-drive-api";

pub fn service_account_view_from_row(row: sqlx::postgres::PgRow) -> ServiceAccountView {
    let oauth_clients = row
        .get::<Option<Uuid>, _>("oauth_client_uuid")
        .map(|id| ServiceAccountClientView {
            id,
            client_id: row.get("client_id"),
            name: row.get("oauth_client_name"),
            created_at: row.get("oauth_client_created_at"),
            revoked_at: row.get("oauth_client_revoked_at"),
            client_assertion_required: row.get("client_assertion_required"),
            client_assertion_public_key_configured: row
                .get("client_assertion_public_key_configured"),
            allowed_scopes: row
                .get::<Option<Vec<String>>, _>("allowed_scopes")
                .unwrap_or_default(),
            allowed_audiences: row
                .get::<Option<Vec<String>>, _>("allowed_audiences")
                .unwrap_or_default(),
            allowed_resources: row
                .get::<Option<Vec<String>>, _>("allowed_resources")
                .unwrap_or_default(),
            required_acr: row
                .get::<Option<String>, _>("required_acr")
                .unwrap_or_else(|| "aal1".to_string()),
        })
        .into_iter()
        .collect();

    ServiceAccountView {
        principal_id: row.get("principal_id"),
        tenant_id: row.get("tenant_id"),
        organization_id: row.get("organization_id"),
        workspace_id: row.get("workspace_id"),
        name: row.get("name"),
        description: row.get("description"),
        role: row.get("role"),
        status: row.get("principal_status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        oauth_clients,
    }
}

pub fn service_account_client_view_from_oauth_result(
    result: &oauth::service::CreateOAuthClientResult,
) -> ServiceAccountClientView {
    ServiceAccountClientView {
        id: result.client.id,
        client_id: result.client.client_id.clone(),
        name: result.client.name.clone(),
        created_at: result.client.created_at,
        revoked_at: None,
        client_assertion_required: result.client.client_assertion_required,
        client_assertion_public_key_configured: result
            .client
            .client_assertion_public_key_configured,
        allowed_scopes: result.policy.allowed_scopes.clone(),
        allowed_audiences: result.policy.allowed_audiences.clone(),
        allowed_resources: result.policy.allowed_resources.clone(),
        required_acr: result.policy.required_acr.clone(),
    }
}

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

pub async fn record_audit_event(
    tx: &mut Transaction<'_, Postgres>,
    access: &WorkspaceAccess,
    action: &str,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), AppError> {
    nvbes_audit::insert_audit_event_tx(
        &mut **tx,
        AuditEventInput {
            tenant_id: required_tenant_id(access)?,
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
            action,
            target_type: "service_account",
            target_id,
            ip,
            user_agent,
            metadata,
        },
    )
    .await
    .map_err(|error| AppError::internal("audit_insert_failed", &format!("{error}")))
}

pub fn new_client_secret() -> String {
    format!(
        "gxo_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

pub fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}
