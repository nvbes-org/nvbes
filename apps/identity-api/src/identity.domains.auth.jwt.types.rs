use chrono::Duration;
use jsonwebtoken::EncodingKey;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domains::auth::keys::KeyBackend;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenConfirmation {
    pub jkt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActorClaim {
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub jti: String,
    pub sid: String,
    pub sub: String,
    pub workspace_id: Option<String>,
    pub workspace_region: Option<String>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub token_type: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub authorization_details: Vec<serde_json::Value>,
    pub acr: Option<String>,
    pub amr: Vec<String>,
    pub client_id: Option<String>,
    pub auth_time: Option<i64>,
    pub iss: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf: Option<TokenConfirmation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<ActorClaim>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_jti: String,
    #[serde(skip_serializing)]
    pub session_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf_jkt: Option<String>,
}

#[derive(Clone)]
pub(crate) enum Signer {
    Kms {
        backend: std::sync::Arc<KeyBackend>,
        kms_key_id: String,
    },
    Local {
        encoding_key: EncodingKey,
    },
}

#[derive(Clone)]
pub struct JwtService {
    pub(crate) signer: Signer,
    pub(crate) kid: String,
    pub(crate) decoding_keys: Vec<(String, jsonwebtoken::DecodingKey)>,
    pub(crate) issuer: String,
    pub(crate) audience: String,
    pub(crate) access_token_expiry: Duration,
    pub(crate) refresh_token_expiry: Duration,
}
