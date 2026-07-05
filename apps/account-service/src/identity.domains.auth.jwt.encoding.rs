use jsonwebtoken::{Algorithm, Header};

use super::types::{JwtService, Signer, TokenClaims};
use crate::http::error::AppError;

impl JwtService {
    pub(crate) fn encode_token(&self, claims: &TokenClaims) -> Result<String, AppError> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());

        let Signer::Local { encoding_key } = &self.signer;
        jsonwebtoken::encode(&header, claims, encoding_key)
            .map_err(|e| AppError::internal("token_generation_failed", e.to_string()))
    }
}
