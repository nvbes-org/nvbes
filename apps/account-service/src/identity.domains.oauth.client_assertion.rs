#[path = "identity.domains.oauth.client_assertion.auth.rs"]
mod auth;
#[path = "identity.domains.oauth.client_assertion.jwk.rs"]
mod jwk;
#[path = "identity.domains.oauth.client_assertion.replay.rs"]
mod replay;
#[cfg(test)]
#[path = "identity.domains.oauth.client_assertion.tests.rs"]
mod tests;
#[path = "identity.domains.oauth.client_assertion.verify.rs"]
mod verify;

use serde::Deserialize;

pub use auth::{assertion_auth, client_id_from_unverified_assertion};
pub use jwk::{is_high_assurance_client_assertion_jwk, is_supported_client_assertion_public_jwk};
pub use verify::verify_private_key_jwt;

pub const CLIENT_ASSERTION_TYPE_JWT_BEARER: &str =
    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";
pub(crate) const MAX_ASSERTION_TTL_SECONDS: i64 = 300;
pub(crate) const CLOCK_SKEW_SECONDS: i64 = 5;

#[derive(Debug, Deserialize)]
pub(crate) struct ClientAssertionIdentityClaims {
    pub iss: String,
    pub sub: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientAssertionClaims {
    pub iss: String,
    pub sub: String,
    pub aud: serde_json::Value,
    pub exp: i64,
    #[serde(default)]
    pub iat: Option<i64>,
    #[serde(default)]
    pub jti: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientAssertionJwk {
    pub kty: String,
    #[serde(default)]
    pub kid: Option<String>,
    #[serde(default)]
    pub alg: Option<String>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
    #[serde(default)]
    pub crv: Option<String>,
    #[serde(default)]
    pub x: Option<String>,
    #[serde(default)]
    pub y: Option<String>,
}
