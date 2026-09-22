use axum::{extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use uuid::Uuid;

use crate::{app::AccountState, config::AccountConfig, error::AccountError};

#[derive(Debug, Clone, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    token_type: String,
    scope: String,
    amr: Vec<String>,
    iss: String,
    aud: String,
    exp: u64,
    iat: u64,
    nbf: u64,
    jti: String,
    sid: String,
}

pub struct TokenVerifier {
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    key_id: String,
}

impl TokenVerifier {
    pub fn new(config: &AccountConfig) -> anyhow::Result<Self> {
        Ok(Self {
            decoding_key: DecodingKey::from_rsa_pem(config.token_public_key_pem.as_bytes())?,
            issuer: config.token_issuer.clone(),
            audience: config.token_audience.clone(),
            key_id: config.token_key_id.clone(),
        })
    }

    fn verify(&self, token: &str) -> Result<Principal, AccountError> {
        let header = decode_header(token).map_err(|_| AccountError::Unauthorized)?;
        if header.alg != Algorithm::RS256
            || header.kid.as_deref() != Some(&self.key_id)
            || header.typ.as_deref() != Some("at+jwt")
        {
            return Err(AccountError::Unauthorized);
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub", "nbf"]);
        let claims = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| AccountError::Unauthorized)?
            .claims;
        if claims.token_type != "access"
            || claims.iss != self.issuer
            || claims.aud != self.audience
            || Uuid::parse_str(&claims.sid).is_err()
            || Uuid::parse_str(&claims.jti).is_err()
            || claims.exp <= claims.iat
            || claims.nbf > claims.iat
        {
            return Err(AccountError::Unauthorized);
        }
        Ok(Principal {
            id: Uuid::parse_str(&claims.sub).map_err(|_| AccountError::Unauthorized)?,
            scopes: claims.scope.split_whitespace().map(str::to_owned).collect(),
            authentication_methods: claims.amr,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub id: Uuid,
    scopes: Vec<String>,
    authentication_methods: Vec<String>,
}

impl Principal {
    pub fn require(&self, scope: &str) -> Result<(), AccountError> {
        self.scopes
            .iter()
            .any(|candidate| candidate == scope)
            .then_some(())
            .ok_or(AccountError::Forbidden)
    }

    pub fn require_step_up(&self) -> Result<(), AccountError> {
        self.authentication_methods
            .iter()
            .any(|method| matches!(method.as_str(), "totp" | "webauthn"))
            .then_some(())
            .ok_or(AccountError::Forbidden)
    }
}

impl FromRequestParts<AccountState> for Principal {
    type Rejection = AccountError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AccountState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AccountError::Unauthorized)?;
        state.tokens.verify(token)
    }
}

#[cfg(test)]
#[path = "account.auth.tests.rs"]
mod tests;
