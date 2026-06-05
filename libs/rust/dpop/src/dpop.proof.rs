use base64::Engine;
use chrono::Utc;
use jsonwebtoken::jwk::{
    AlgorithmParameters, CommonParameters, EllipticCurve, EllipticCurveKeyParameters,
    EllipticCurveKeyType, Jwk as JwtJwk,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::keys::{DpopKeyPair, Jwk};

#[derive(Debug, thiserror::Error)]
pub enum DpopError {
    #[error("invalid DPoP proof format: {0}")]
    InvalidProof(String),
    #[error("missing DPoP header")]
    MissingHeader,
    #[error("DPoP proof expired (iat too old)")]
    Expired,
    #[error("DPoP proof issued in the future")]
    FutureIat,
    #[error("HTTP method mismatch: expected {expected}, got {actual}")]
    HtmMismatch { expected: String, actual: String },
    #[error("HTTP URL mismatch: expected {expected}, got {actual}")]
    HtuMismatch { expected: String, actual: String },
    #[error("access token hash mismatch")]
    AthMismatch,
    #[error("missing JWK in DPoP proof header")]
    MissingJwk,
    #[error("invalid JWK in DPoP proof: {0}")]
    InvalidJwk(#[from] crate::keys::DpopKeyError),
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("missing nonce in DPoP proof (server requires nonces)")]
    MissingNonce,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DpopProofClaims {
    pub jti: String,
    pub htm: String,
    pub htu: String,
    pub iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Debug)]
pub struct DpopProof {
    pub claims: DpopProofClaims,
    pub jwk: Jwk,
}

fn jwk_to_jwt_jwk(jwk: &Jwk) -> JwtJwk {
    JwtJwk {
        common: CommonParameters::default(),
        algorithm: AlgorithmParameters::EllipticCurve(EllipticCurveKeyParameters {
            key_type: EllipticCurveKeyType::EC,
            curve: EllipticCurve::P256,
            x: jwk.x.clone(),
            y: jwk.y.clone(),
        }),
    }
}

fn jwt_jwk_to_jwk(jwt_jwk: &JwtJwk) -> Result<Jwk, DpopError> {
    match &jwt_jwk.algorithm {
        AlgorithmParameters::EllipticCurve(params) => {
            let crv = match params.curve {
                EllipticCurve::P256 => "P-256",
                EllipticCurve::P384 => "P-384",
                EllipticCurve::P521 => "P-521",
                EllipticCurve::Ed25519 => "Ed25519",
            };
            Ok(Jwk {
                kty: "EC".to_string(),
                crv: crv.to_string(),
                x: params.x.clone(),
                y: params.y.clone(),
            })
        }
        _ => Err(DpopError::InvalidProof(
            "unsupported key type in JWK".into(),
        )),
    }
}

pub fn create_dpop_proof(
    key_pair: &DpopKeyPair,
    htm: &str,
    htu: &str,
    access_token: Option<&str>,
    nonce: Option<&str>,
) -> Result<String, DpopError> {
    let jti = uuid::Uuid::new_v4().to_string();
    let iat = Utc::now().timestamp();

    let ath = access_token.map(|token| {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let digest = hasher.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
    });

    let claims = DpopProofClaims {
        jti,
        htm: htm.to_uppercase(),
        htu: htu.to_string(),
        iat,
        ath,
        nonce: nonce.map(|n| n.to_string()),
    };

    let mut header = Header::new(Algorithm::ES256);
    header.typ = Some("dpop+jwt".to_string());
    header.jwk = Some(jwk_to_jwt_jwk(&key_pair.jwk));

    let signing_key = p256_to_jwt_signing_key(&key_pair.signing_key)?;

    let token = jsonwebtoken::encode(&header, &claims, &signing_key)?;
    Ok(token)
}

pub fn verify_dpop_proof(
    proof_str: &str,
    expected_htm: &str,
    expected_htu: &str,
    expected_ath: Option<&str>,
    max_iat_skew_secs: i64,
) -> Result<DpopProof, DpopError> {
    let header = jsonwebtoken::decode_header(proof_str)?;

    if header.typ.as_deref() != Some("dpop+jwt") {
        return Err(DpopError::InvalidProof("typ must be dpop+jwt".into()));
    }
    if header.alg != Algorithm::ES256 {
        return Err(DpopError::InvalidProof("alg must be ES256".into()));
    }

    let jwt_jwk = header.jwk.as_ref().ok_or(DpopError::MissingJwk)?;
    let jwk = jwt_jwk_to_jwk(jwt_jwk)?;

    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = false;
    validation.set_required_spec_claims(&["jti", "htm", "htu", "iat"]);
    validation.required_spec_claims.remove("sub");
    validation.required_spec_claims.remove("exp");

    let token_data =
        jsonwebtoken::decode::<DpopProofClaims>(proof_str, &decoding_key, &validation)?;
    let claims = token_data.claims;

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
        let mut hasher = Sha256::new();
        hasher.update(expected.as_bytes());
        let digest = hasher.finalize();
        let expected_ath = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

        match &claims.ath {
            Some(actual) if actual == &expected_ath => {}
            _ => return Err(DpopError::AthMismatch),
        }
    }

    Ok(DpopProof { claims, jwk })
}

fn p256_to_jwt_signing_key(
    signing_key: &p256::ecdsa::SigningKey,
) -> Result<EncodingKey, DpopError> {
    use p256::pkcs8::EncodePrivateKey;
    let der = signing_key
        .to_pkcs8_der()
        .map_err(|e| DpopError::InvalidProof(format!("failed to encode private key: {}", e)))?;
    Ok(EncodingKey::from_ec_der(der.as_bytes()))
}

pub fn compute_ath(access_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(access_token.as_bytes());
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::generate_key_pair;

    #[test]
    fn create_and_verify_dpop_proof_roundtrip() {
        let pair = generate_key_pair();
        let proof = create_dpop_proof(&pair, "GET", "https://api.example.com/resource", None, None)
            .expect("proof creation should succeed");

        let verified =
            verify_dpop_proof(&proof, "GET", "https://api.example.com/resource", None, 300)
                .expect("verification should succeed");

        assert_eq!(verified.claims.htm, "GET");
        assert_eq!(verified.claims.htu, "https://api.example.com/resource");
    }

    #[test]
    fn dpop_proof_with_ath_binds_to_token() {
        let pair = generate_key_pair();
        let token = "my-access-token-value";
        let proof = create_dpop_proof(
            &pair,
            "POST",
            "https://api.example.com/data",
            Some(token),
            None,
        )
        .expect("proof creation should succeed");

        verify_dpop_proof(
            &proof,
            "POST",
            "https://api.example.com/data",
            Some(token),
            300,
        )
        .expect("verification with correct ath should succeed");
    }

    #[test]
    fn dpop_proof_rejects_wrong_ath() {
        let pair = generate_key_pair();
        let proof = create_dpop_proof(
            &pair,
            "GET",
            "https://api.example.com/resource",
            Some("token-a"),
            None,
        )
        .expect("proof creation should succeed");

        let result = verify_dpop_proof(
            &proof,
            "GET",
            "https://api.example.com/resource",
            Some("token-b"),
            300,
        );
        assert!(result.is_err());
    }

    #[test]
    fn dpop_proof_rejects_wrong_htm() {
        let pair = generate_key_pair();
        let proof = create_dpop_proof(&pair, "GET", "https://api.example.com/resource", None, None)
            .expect("proof creation should succeed");

        let result = verify_dpop_proof(
            &proof,
            "POST",
            "https://api.example.com/resource",
            None,
            300,
        );
        assert!(result.is_err());
    }

    #[test]
    fn dpop_proof_rejects_wrong_htu() {
        let pair = generate_key_pair();
        let proof = create_dpop_proof(&pair, "GET", "https://api.example.com/resource", None, None)
            .expect("proof creation should succeed");

        let result = verify_dpop_proof(&proof, "GET", "https://api.example.com/other", None, 300);
        assert!(result.is_err());
    }
}
