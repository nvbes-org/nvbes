#[path = "dpop.proof.create.rs"]
mod create;
#[path = "dpop.proof.jwk.rs"]
mod jwk;
#[cfg(test)]
#[path = "dpop.proof.tests.rs"]
mod tests;
#[path = "dpop.proof.verify.rs"]
mod verify;

use serde::{Deserialize, Serialize};

use crate::keys::Jwk;

#[derive(Debug, thiserror::Error)]
pub enum DpopError {
    #[error("invalid DPoP proof format: {0}")]
    InvalidProof(String),
    #[error("missing DPoP header")]
    MissingHeader,
    #[error("DPoP proof expired (iat too old)")]
    Expired,
    #[error("DPoP proof issued in the future")]
    FutureIat,
    #[error("HTTP method mismatch: expected {expected}, got {actual}")]
    HtmMismatch { expected: String, actual: String },
    #[error("HTTP URL mismatch: expected {expected}, got {actual}")]
    HtuMismatch { expected: String, actual: String },
    #[error("access token hash mismatch")]
    AthMismatch,
    #[error("missing JWK in DPoP proof header")]
    MissingJwk,
    #[error("invalid JWK in DPoP proof: {0}")]
    InvalidJwk(#[from] crate::keys::DpopKeyError),
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("missing nonce in DPoP proof (server requires nonces)")]
    MissingNonce,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DpopProofClaims {
    pub jti: String,
    pub htm: String,
    pub htu: String,
    pub iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Debug)]
pub struct DpopProof {
    pub claims: DpopProofClaims,
    pub jwk: Jwk,
}

pub use create::create_dpop_proof;
pub use verify::{compute_ath, verify_dpop_proof};
