use axum::{extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use uuid::Uuid;

use crate::{app::BillingState, config::BillingConfig, error::BillingError};

#[derive(Debug, Clone, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    scope: Option<String>,
    amr: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct TokenVerifier {
    decoding_key: Option<DecodingKey>,
}

impl TokenVerifier {
    pub fn new(config: &BillingConfig) -> anyhow::Result<Self> {
        let decoding_key = match &config.identity_public_key_pem {
            Some(pem) => Some(DecodingKey::from_rsa_pem(pem.as_bytes())?),
            None => None,
        };
        Ok(Self { decoding_key })
    }

    pub fn verify(&self, token: &str) -> Result<BillingPrincipal, BillingError> {
        if let Some(key) = &self.decoding_key {
            let header = decode_header(token).map_err(|_| BillingError::Unauthorized)?;
            if header.alg != Algorithm::RS256 {
                return Err(BillingError::Unauthorized);
            }
            let mut validation = Validation::new(Algorithm::RS256);
            validation.validate_aud = false;
            validation.set_required_spec_claims(&["sub"]);
            let claims = decode::<AccessTokenClaims>(token, key, &validation)
                .map_err(|_| BillingError::Unauthorized)?
                .claims;
            let id = Uuid::parse_str(&claims.sub).map_err(|_| BillingError::Unauthorized)?;
            let scopes = claims
                .scope
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            let amr = claims.amr.unwrap_or_default();
            Ok(BillingPrincipal { id, scopes, amr })
        } else {
            // Development / test bypass: accept bearer token formatted as uuid or test prefix
            let stripped = token.strip_prefix("test-").unwrap_or(token);
            let id = Uuid::parse_str(stripped).unwrap_or_else(|_| Uuid::nil());
            Ok(BillingPrincipal {
                id,
                scopes: vec![
                    "billing:read".into(),
                    "billing:write".into(),
                    "account:read".into(),
                ],
                amr: vec!["pwd".into()],
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct BillingPrincipal {
    id: Uuid,
    scopes: Vec<String>,
    amr: Vec<String>,
}

impl BillingPrincipal {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn has_mfa(&self) -> bool {
        self.amr.iter().any(|m| m == "totp" || m == "webauthn")
    }

    pub fn require_scope(&self, scope: &str) -> Result<(), BillingError> {
        if self
            .scopes
            .iter()
            .any(|s| s == scope || s == "billing:admin")
        {
            Ok(())
        } else {
            Err(BillingError::Forbidden)
        }
    }
}

impl FromRequestParts<BillingState> for BillingPrincipal {
    type Rejection = BillingError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &BillingState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(BillingError::Unauthorized)?;
        state.tokens.verify(token)
    }
}

pub struct OperatorAuth;

impl FromRequestParts<BillingState> for OperatorAuth {
    type Rejection = BillingError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &BillingState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(BillingError::Unauthorized)?;

        if let Some(expected) = &state.config.operator_token {
            if nvbes_billing::stripe::constant_time_eq(token.as_bytes(), expected.as_bytes()) {
                return Ok(OperatorAuth);
            }
        }
        Err(BillingError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn principal_scopes_and_mfa_verification() {
        let principal = BillingPrincipal {
            id: Uuid::new_v4(),
            scopes: vec!["billing:write".into()],
            amr: vec!["totp".into()],
        };
        assert_eq!(principal.id(), principal.id);
        assert!(principal.has_mfa());
        assert!(principal.require_scope("billing:write").is_ok());
        assert!(principal.require_scope("billing:admin").is_err());
    }
}
