use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct JitProvisioningResponse {
    pub principal_id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub linked_identity_id: Uuid,
    pub created_account: bool,
    pub created_membership: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InboundFederationResponse {
    pub principal_id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub session_id: Uuid,
    pub token_type: String,
    pub expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
    pub provider_type: String,
    pub provider_name: String,
    pub linked_identity_id: Uuid,
    pub created_account: bool,
    pub created_membership: bool,
}

#[derive(Debug, ToSchema)]
pub struct JitProvisioningInput {
    pub email: String,
    pub username: String,
    pub provider_type: String,
    pub provider_id: String,
    pub subject: String,
    pub email_verified: bool,
}

#[derive(Debug, ToSchema)]
pub struct InboundFederationInput {
    pub provider_id: Uuid,
    pub email: String,
    pub username: String,
    pub subject: String,
    pub email_verified: bool,
}
