use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{EnterpriseModuleGrant, EnterpriseRole};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseInvitationInput {
    pub emails: Vec<String>,
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub workspace_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAccessUpdateInput {
    pub role: EnterpriseRole,
    pub module_grants: Vec<EnterpriseModuleGrant>,
    pub workspace_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAdminElevationInput {
    pub duration_minutes: Option<i64>,
    pub reason: Option<String>,
    pub procedure_reference: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAuditReasonInput {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseBreakGlassInput {
    pub reason: String,
    pub procedure_reference: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSuspendInput {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseReactivateInput {
    pub reason: String,
    pub module_grants: Option<Vec<EnterpriseModuleGrant>>,
    pub workspace_ids: Option<Vec<Uuid>>,
}
