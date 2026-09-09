use nvbes_dpop::{jwk_thumbprint, verify_dpop_proof};

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

#[cfg(test)]
#[path = "identity.oauth.dpop.tests.rs"]
mod tests;
