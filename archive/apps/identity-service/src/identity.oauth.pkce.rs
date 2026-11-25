use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use super::error::OAuthError;

pub fn validate_challenge(challenge: &str, method: &str) -> Result<(), OAuthError> {
    if method != "S256" || !is_sha256_base64url(challenge) {
        return Err(OAuthError::InvalidRequest);
    }
    Ok(())
}

pub fn verify(challenge: &str, verifier: &str) -> Result<(), OAuthError> {
    if !(43..=128).contains(&verifier.len())
        || !verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
    {
        return Err(OAuthError::InvalidGrant);
    }
    let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    if expected != challenge {
        return Err(OAuthError::InvalidGrant);
    }
    Ok(())
}

pub fn is_sha256_base64url(value: &str) -> bool {
    value.len() == 43
        && URL_SAFE_NO_PAD
            .decode(value)
            .is_ok_and(|decoded| decoded.len() == 32)
}
