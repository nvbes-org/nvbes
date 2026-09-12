use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{Validation, decode, decode_header};
use sha2::{Digest, Sha256};

use super::{
    DpopError, DpopProof, DpopProofClaims,
    jwk::{DPOP_ALGORITHM, decoding_key_for_jwk, jwt_jwk_to_jwk},
};

pub fn verify_dpop_proof(
    proof_str: &str,
    expected_htm: &str,
    expected_htu: &str,
    expected_ath: Option<&str>,
    max_iat_skew_secs: i64,
) -> Result<DpopProof, DpopError> {
    let header = decode_header(proof_str)?;

    if header.typ.as_deref() != Some("dpop+jwt") {
        return Err(DpopError::InvalidProof("typ must be dpop+jwt".into()));
    }
    if header.alg != DPOP_ALGORITHM {
        return Err(DpopError::InvalidProof("alg must be ES256".into()));
    }

    let jwt_jwk = header.jwk.as_ref().ok_or(DpopError::MissingJwk)?;
    let jwk = jwt_jwk_to_jwk(jwt_jwk)?;
    let decoding_key = decoding_key_for_jwk(&jwk)?;

    let mut validation = Validation::new(DPOP_ALGORITHM);
    validation.validate_exp = false;
    validation.set_required_spec_claims(&["jti", "htm", "htu", "iat"]);
    validation.required_spec_claims.remove("sub");
    validation.required_spec_claims.remove("exp");

    let claims = decode::<DpopProofClaims>(proof_str, &decoding_key, &validation)?.claims;

    let now = Utc::now().timestamp();
    if claims.iat < now - max_iat_skew_secs {
        return Err(DpopError::Expired);
    }
    if claims.iat > now + max_iat_skew_secs {
        return Err(DpopError::FutureIat);
    }
    if claims.htm != expected_htm {
        return Err(DpopError::HtmMismatch {
            expected: expected_htm.to_string(),
            actual: claims.htm,
        });
    }
    if claims.htu != expected_htu {
        return Err(DpopError::HtuMismatch {
            expected: expected_htu.to_string(),
            actual: claims.htu,
        });
    }

    if let Some(expected) = expected_ath {
        let expected_ath = compute_ath(expected);
        match &claims.ath {
            Some(actual) if actual == &expected_ath => {}
            _ => return Err(DpopError::AthMismatch),
        }
    }

    Ok(DpopProof { claims, jwk })
}

pub fn compute_ath(access_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(access_token.as_bytes());
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}
