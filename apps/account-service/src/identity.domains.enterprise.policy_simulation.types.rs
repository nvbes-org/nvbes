use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePolicySimulationInput {
    pub workspace_id: Uuid,
    pub subject: EnterprisePolicySimulationSubject,
    pub action: String,
    pub resource: Option<EnterprisePolicySimulationResourceInput>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "subject_type", rename_all = "snake_case")]
pub enum EnterprisePolicySimulationSubject {
    User { user_id: Uuid },
    Client { client_id: String },
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePolicySimulationResourceInput {
    #[serde(default)]
    pub owns_resource: bool,
    #[serde(default)]
    pub member_share_links_enabled: bool,
    pub target_role: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePolicySimulationResponse {
    pub decision: EnterprisePolicySimulationDecision,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct EnterprisePolicySimulationDecision {
    pub allowed: bool,
    pub reason: String,
    pub action: String,
    pub workspace_id: Uuid,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_label: String,
    pub role: Option<String>,
    pub requires_step_up: bool,
}

impl Default for EnterprisePolicySimulationResourceInput {
    fn default() -> Self {
        Self {
            owns_resource: false,
            member_share_links_enabled: false,
            target_role: None,
        }
    }
}
