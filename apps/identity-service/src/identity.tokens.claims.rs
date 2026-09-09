use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofConfirmation {
    pub jkt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub token_type: String,
    pub client_id: String,
    pub scope: String,
    pub amr: Vec<String>,
    pub auth_time: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_up_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_up_expires_at: Option<u64>,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub nbf: u64,
    pub jti: String,
    pub sid: String,
    pub grant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf: Option<ProofConfirmation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    pub iss: String,
    pub sub: String,
    /// The OIDC client, distinct from the resource server access-token audience.
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub auth_time: u64,
    pub amr: Vec<String>,
    pub nonce: String,
    pub sid: String,
    pub at_hash: String,
}

/// Secrets deliberately have no Debug implementation.
#[derive(Serialize)]
pub struct TokenSet {
    pub access_token: String,
    pub id_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonWebKeySet {
    pub keys: Vec<JsonWebKey>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonWebKey {
    pub kid: String,
    pub kty: &'static str,
    #[serde(rename = "use")]
    pub usage: &'static str,
    pub alg: &'static str,
    pub n: String,
    pub e: String,
}
