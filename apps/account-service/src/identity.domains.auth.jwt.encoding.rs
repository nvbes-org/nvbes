use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, Header};
use serde::Serialize;

use super::types::{JwtService, Signer, TokenClaims};
use crate::http::error::AppError;

impl JwtService {
    pub(crate) async fn encode_token(&self, claims: &TokenClaims) -> Result<String, AppError> {
        let token_type = match claims.token_type.as_str() {
            "access" => "at+jwt",
            "refresh" => "refresh+jwt",
            _ => "JWT",
        };
        self.encode_typed_claims(claims, Some(token_type)).await
    }

    pub(crate) async fn encode_claims<T: Serialize>(&self, claims: &T) -> Result<String, AppError> {
        self.encode_typed_claims(claims, None).await
    }

    pub(crate) async fn encode_typed_claims<T: Serialize>(
        &self,
        claims: &T,
        token_type: Option<&str>,
    ) -> Result<String, AppError> {
        let mut header = Header::new(Algorithm::PS256);
        header.kid = Some(self.kid.clone());
        header.typ = token_type.map(str::to_string);

        match &self.signer {
            Signer::Local { encoding_key } => jsonwebtoken::encode(&header, claims, encoding_key)
                .map_err(|e| AppError::internal("token_generation_failed", e.to_string())),
            Signer::Kms { signer } => {
                let encoded_header =
                    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(|error| {
                        AppError::internal("token_generation_failed", error.to_string())
                    })?);
                let encoded_claims =
                    URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).map_err(|error| {
                        AppError::internal("token_generation_failed", error.to_string())
                    })?);
                let signing_input = format!("{encoded_header}.{encoded_claims}");
                let signature = signer.sign_ps256(signing_input.as_bytes()).await?;
                Ok(format!(
                    "{signing_input}.{}",
                    URL_SAFE_NO_PAD.encode(signature)
                ))
            }
        }
    }
}
