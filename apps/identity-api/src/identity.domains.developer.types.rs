use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperContextResponse {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperOverviewResponse {
    pub tenant_id: Uuid,
    pub oauth_clients: i64,
    pub marketplace_pending: i64,
    pub high_risk_scopes: i64,
    pub failed_webhook_deliveries: i64,
    pub unhealthy_integrations: i64,
    pub active_sandboxes: i64,
}
