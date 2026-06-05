use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;

#[derive(Deserialize, ToSchema)]
pub struct IdentifierRequest {
    pub email: String,
    pub turnstile_token: Option<String>,
    pub device_fingerprint: Option<serde_json::Value>,
    pub bot_guard: Option<crate::domains::auth::bot_guard::BotGuardProof>,
    pub bot_signals: Option<crate::domains::auth::bot_signals::BotSignals>,
    #[serde(default)]
    pub pow_nonce: Option<String>,
    #[serde(default)]
    pub pow_solution: Option<String>,
    #[serde(default)]
    pub decoy_link_clicked: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct IdentifierResult {
    pub next_step: String,
    pub state_token: Uuid,
    pub available_methods: Option<Vec<String>>,
}

#[derive(Deserialize, ToSchema)]
pub struct PwdRequest {
    pub state_token: Uuid,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct MfaRequest {
    pub state_token: Uuid,
    pub totp_code: Option<String>,
    pub recovery_code: Option<String>,
    #[schema(value_type = Object)]
    pub webauthn_response: Option<PublicKeyCredential>,
    pub webauthn_challenge_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct WebauthnStartRequest {
    pub state_token: Uuid,
}
