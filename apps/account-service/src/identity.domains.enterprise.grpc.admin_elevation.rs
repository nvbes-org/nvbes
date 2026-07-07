use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domains::enterprise::types::{EnterpriseAdminElevationInput, EnterpriseRole},
    grpc_pb::nvbes::enterprise::v1::{AdminElevationAuthorization, AuthorizeAdminElevationRequest},
    http::error::AppError,
};

pub struct AuthorizeAdminElevationCommand {
    pub tenant_id: Uuid,
    pub actor_principal_id: Uuid,
    pub base_role: EnterpriseRole,
    pub input: EnterpriseAdminElevationInput,
    pub break_glass: bool,
    pub break_glass_procedure: Option<(String, String)>,
    pub step_up_expires_at: DateTime<Utc>,
    pub session_expires_at: DateTime<Utc>,
}

pub async fn authorize_admin_elevation(
    command: AuthorizeAdminElevationCommand,
) -> Result<AdminElevationAuthorization, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .authorize_admin_elevation(AuthorizeAdminElevationRequest {
            context: Some(super::request_context(
                command.tenant_id,
                command.actor_principal_id,
            )),
            tenant_id: command.tenant_id.to_string(),
            base_role: role_label(&command.base_role).to_string(),
            duration_minutes: command
                .input
                .duration_minutes
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or_default(),
            step_up_expires_at: command.step_up_expires_at.to_rfc3339(),
            session_expires_at: command.session_expires_at.to_rfc3339(),
            break_glass: command.break_glass,
            break_glass_reason: command
                .break_glass_procedure
                .as_ref()
                .map(|(reason, _)| reason.clone())
                .unwrap_or_default(),
            break_glass_procedure_reference: command
                .break_glass_procedure
                .as_ref()
                .map(|(_, procedure_reference)| procedure_reference.clone())
                .unwrap_or_default(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

fn role_label(role: &EnterpriseRole) -> &'static str {
    match role {
        EnterpriseRole::Owner => "owner",
        EnterpriseRole::Admin => "admin",
        EnterpriseRole::Member => "member",
        EnterpriseRole::Viewer => "viewer",
    }
}
