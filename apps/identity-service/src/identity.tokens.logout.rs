use super::{ACCESS_TOKEN_TTL_SECONDS, TokenService};
use crate::{tokens_claims::IdTokenClaims, tokens_error::TokenError, tokens_policy};
use chrono::Utc;
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use uuid::Uuid;

/// Signed identification only, never an authentication or revocation capability.
/// The logout handler must separately bind it to the browser's current/recent
/// session and obtain confirmation before changing any session state.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogoutHint {
    pub client_id: String,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub issued_at: u64,
}

impl TokenService {
    /// Logout permits an expired ID Token; access-token verification does not.
    /// Retired signing keys remain rejected regardless of token expiry.
    pub fn verify_logout_hint(&self, token: &str) -> Result<LogoutHint, TokenError> {
        if token.len() > 16_384 {
            return Err(TokenError::InvalidToken);
        }
        let now = Utc::now().timestamp() as u64;
        let header = decode_header(token)?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some("JWT")
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
        {
            return Err(TokenError::InvalidToken);
        }
        let key = self
            .keys
            .decoding_key(header.kid.as_deref().ok_or(TokenError::InvalidToken)?, now)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
        validation.validate_exp = false;
        // The signed audience identifies the RP. Its current registration is
        // checked by LogoutRequest, rather than treating a resource as audience.
        validation.validate_aud = false;
        validation.leeway = 0;
        let claims = decode::<IdTokenClaims>(token, key, &validation)?.claims;
        if claims.iat > now
            || claims.exp <= claims.iat
            || claims.exp - claims.iat > ACCESS_TOKEN_TTL_SECONDS
            || claims.auth_time > claims.iat
            || claims.aud.is_empty()
            || claims.aud.len() > 128
        {
            return Err(TokenError::InvalidToken);
        }
        tokens_policy::validate_amr(&claims.amr)?;
        Ok(LogoutHint {
            client_id: claims.aud,
            principal_id: Uuid::parse_str(&claims.sub).map_err(|_| TokenError::InvalidToken)?,
            session_id: Uuid::parse_str(&claims.sid).map_err(|_| TokenError::InvalidToken)?,
            issued_at: claims.iat,
        })
    }
}

#[cfg(test)]
#[path = "identity.tokens.logout.tests.rs"]
mod tests;
