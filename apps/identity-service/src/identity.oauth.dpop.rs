use nvbes_dpop::{jwk_thumbprint, verify_dpop_proof};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use super::error::OAuthError;

/// Verify the proof before passing its thumbprint into code exchange. The
/// caller must persist/check the proof `jti` for replay at the same boundary.
pub fn verify_code_proof(proof: &str, method: &str, url: &str) -> Result<String, OAuthError> {
    if proof.len() > 16_384 || method.is_empty() || url.is_empty() {
        return Err(OAuthError::InvalidRequest);
    }
    let verified =
        verify_dpop_proof(proof, method, url, None, 300).map_err(|_| OAuthError::InvalidRequest)?;
    Ok(jwk_thumbprint(&verified.jwk))
}

pub async fn verify_and_consume_code_proof(
    db: &PgPool,
    proof: &str,
    method: &str,
    url: &str,
) -> Result<String, OAuthError> {
    if proof.len() > 16_384 || method.is_empty() || url.is_empty() {
        return Err(OAuthError::InvalidRequest);
    }
    let verified =
        verify_dpop_proof(proof, method, url, None, 300).map_err(|_| OAuthError::InvalidRequest)?;
    let jkt = jwk_thumbprint(&verified.jwk);
    let jti_hash = Sha256::digest(verified.claims.jti.as_bytes()).to_vec();
    let jkt_hash = Sha256::digest(jkt.as_bytes()).to_vec();
    let inserted: Option<Vec<u8>> = sqlx::query_scalar(
        "INSERT INTO identity_dpop_replay_keys(jti_hash,jkt_hash,expires_at) VALUES($1,$2,clock_timestamp()+interval '5 minutes') ON CONFLICT DO NOTHING RETURNING jti_hash",
    )
    .bind(jti_hash)
    .bind(jkt_hash)
    .fetch_optional(db)
    .await
    .map_err(|_| OAuthError::Unavailable)?;
    inserted.map(|_| jkt).ok_or(OAuthError::InvalidRequest)
}

#[cfg(test)]
#[path = "identity.oauth.dpop.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.dpop.database.tests.rs"]
mod database_tests;
