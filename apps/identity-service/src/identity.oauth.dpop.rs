use chrono::{DateTime, Utc};
use nvbes_dpop::{jwk_thumbprint, verify_dpop_proof};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};

use super::error::OAuthError;

const PROOF_SKEW_SECONDS: i64 = 300;

pub(crate) struct VerifiedTokenProof {
    jkt: String,
    jti_hash: Vec<u8>,
    expires_at: DateTime<Utc>,
}

impl VerifiedTokenProof {
    pub(crate) fn thumbprint(&self) -> &str {
        &self.jkt
    }

    /// Consume inside the caller's transaction so proof and token state commit together.
    pub(crate) async fn consume(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), OAuthError> {
        if self.expires_at <= Utc::now() {
            return Err(OAuthError::InvalidDpopProof);
        }
        let inserted: Option<Vec<u8>> = sqlx::query_scalar(
            "INSERT INTO identity_dpop_replay_keys(jti_hash,jkt_hash,expires_at) VALUES($1,$2,$3) ON CONFLICT DO NOTHING RETURNING jti_hash",
        )
        .bind(&self.jti_hash)
        .bind(Sha256::digest(self.jkt.as_bytes()).to_vec())
        .bind(self.expires_at)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| OAuthError::Unavailable)?;
        inserted.map(|_| ()).ok_or(OAuthError::InvalidDpopProof)
    }
}

pub(crate) fn verify_token_proof(
    proof: &str,
    method: &str,
    url: &str,
) -> Result<VerifiedTokenProof, OAuthError> {
    verify_proof(proof, method, url, None)
}

pub(crate) fn verify_resource_proof(
    proof: &str,
    method: &str,
    url: &str,
    token: &str,
) -> Result<VerifiedTokenProof, OAuthError> {
    verify_proof(proof, method, url, Some(token))
}

fn verify_proof(
    proof: &str,
    method: &str,
    url: &str,
    token: Option<&str>,
) -> Result<VerifiedTokenProof, OAuthError> {
    if proof.is_empty() || proof.len() > 16_384 || method.is_empty() || url.is_empty() {
        return Err(OAuthError::InvalidDpopProof);
    }
    let verified = verify_dpop_proof(proof, method, url, token, PROOF_SKEW_SECONDS)
        .map_err(|_| OAuthError::InvalidDpopProof)?;
    if verified.claims.jti.is_empty() || verified.claims.jti.len() > 256 {
        return Err(OAuthError::InvalidDpopProof);
    }
    // Future-dated proofs remain valid until iat + skew, inclusive.
    let expires_at = DateTime::from_timestamp(verified.claims.iat + PROOF_SKEW_SECONDS + 1, 0)
        .ok_or(OAuthError::InvalidDpopProof)?;
    Ok(VerifiedTokenProof {
        jkt: jwk_thumbprint(&verified.jwk),
        jti_hash: Sha256::digest(verified.claims.jti.as_bytes()).to_vec(),
        expires_at,
    })
}

/// Cryptographic validation only; token boundaries must also consume the proof.
pub fn verify_code_proof(proof: &str, method: &str, url: &str) -> Result<String, OAuthError> {
    Ok(verify_token_proof(proof, method, url)?.jkt)
}

pub async fn verify_and_consume_code_proof(
    db: &PgPool,
    proof: &str,
    method: &str,
    url: &str,
) -> Result<String, OAuthError> {
    let verified = verify_token_proof(proof, method, url)?;
    let mut tx = db.begin().await.map_err(|_| OAuthError::Unavailable)?;
    verified.consume(&mut tx).await?;
    tx.commit().await.map_err(|_| OAuthError::Unavailable)?;
    Ok(verified.jkt)
}

#[cfg(test)]
#[path = "identity.oauth.dpop.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.dpop.database.tests.rs"]
mod database_tests;
