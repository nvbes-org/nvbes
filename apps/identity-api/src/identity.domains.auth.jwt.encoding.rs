use base64::Engine;
use jsonwebtoken::{Algorithm, Header};
use sha2::{Digest, Sha256};

use super::types::{JwtService, Signer, TokenClaims};
use crate::http::error::AppError;

impl JwtService {
    pub(crate) fn encode_token(&self, claims: &TokenClaims) -> Result<String, AppError> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());

        match &self.signer {
            Signer::Local { encoding_key } => jsonwebtoken::encode(&header, claims, encoding_key)
                .map_err(|e| AppError::internal("token_generation_failed", &e.to_string())),
            Signer::Kms {
                backend,
                kms_key_id,
            } => {
                let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(serde_json::to_vec(&header)?);
                let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(serde_json::to_vec(claims)?);
                let signing_input = format!("{}.{}", header_b64, payload_b64);

                let mut hasher = Sha256::new();
                hasher.update(signing_input.as_bytes());
                let digest = hasher.finalize();

                let client = backend.kms_client().ok_or_else(|| {
                    AppError::internal("no_kms_client", "KMS backend has no client")
                })?;

                let sign_resp = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(client.sign(kms_key_id, &digest))
                })
                .map_err(|e| AppError::internal("kms_signing_failed", &e.to_string()))?;

                let sig_b64 = sign_resp.signature;
                Ok(format!("{}.{}.{}", header_b64, payload_b64, sig_b64))
            }
        }
    }
}
