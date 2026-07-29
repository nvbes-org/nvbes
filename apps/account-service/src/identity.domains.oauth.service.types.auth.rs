use uuid::Uuid;

use crate::domains::oauth::rar::AuthorizationDetails;

#[derive(Debug)]
pub struct CreateAuthorizationCodeInput {
    pub client_id: String,
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub scope: String,
    pub redirect_uri: String,
    pub nonce: Option<String>,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub authorization_details: AuthorizationDetails,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub consent_action: Option<String>,
    pub dpop_jkt: Option<String>,
}

#[derive(Debug)]
pub struct ExchangeCodeInput {
    pub code: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_assertion_verified: bool,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub token_confirmation: Option<crate::domains::auth::jwt::TokenConfirmation>,
}

#[derive(Debug, Clone)]
pub struct ClientAssertionAuthentication {
    pub assertion_type: String,
    pub assertion: String,
}

#[derive(Debug, Clone)]
pub struct ClientAuthentication {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_assertion: Option<ClientAssertionAuthentication>,
    pub client_assertion_verified: bool,
}

impl ClientAuthentication {
    pub fn uses_private_key_jwt(&self) -> bool {
        self.client_assertion.is_some()
    }
}

#[derive(Debug)]
pub struct AuthCodeRecord {
    pub user_id: Uuid,
    pub client_session_id: Option<Uuid>,
    pub scope: String,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub authorization_details: AuthorizationDetails,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub client_id: String,
    pub client_uuid: Uuid,
}

#[derive(Debug)]
pub struct TokenExchangeInput {
    pub subject_token: String,
    pub subject_token_type: String,
    pub actor_token: Option<String>,
    pub actor_token_type: Option<String>,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_assertion_verified: bool,
    pub scope: Option<String>,
    pub audience: Option<String>,
    pub resource: Option<String>,
    pub requested_token_type: Option<String>,
}
