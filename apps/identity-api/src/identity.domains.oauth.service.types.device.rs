use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceAuthorizationInput {
    pub client_id: String,
    pub scope: String,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceAuthorizationView {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: i64,
    pub interval: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExchangeDeviceCodeInput {
    pub client_id: String,
    pub device_code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceVerificationInput {
    pub user_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceVerificationView {
    pub client_name: String,
    pub scope: Vec<String>,
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceApprovalInput {
    pub user_code: String,
    pub workspace_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub consent_action: Option<String>,
}
