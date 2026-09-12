use jsonwebtoken::jwk::{
    AlgorithmParameters, CommonParameters, EllipticCurve, EllipticCurveKeyParameters,
    EllipticCurveKeyType, Jwk as JwtJwk,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};

use crate::keys::Jwk;

use super::DpopError;

pub(super) fn jwk_to_jwt_jwk(jwk: &Jwk) -> JwtJwk {
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

pub(super) fn jwt_jwk_to_jwk(jwt_jwk: &JwtJwk) -> Result<Jwk, DpopError> {
    match &jwt_jwk.algorithm {
        AlgorithmParameters::EllipticCurve(params) => {
            let crv = match params.curve {
                EllipticCurve::P256 => "P-256",
                EllipticCurve::P384 => "P-384",
                EllipticCurve::P521 => "P-521",
                EllipticCurve::Ed25519 => "Ed25519",
                _ => {
                    return Err(DpopError::InvalidProof(
                        "unsupported elliptic curve in JWK".into(),
                    ));
                }
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

pub(super) fn p256_to_jwt_signing_key(
    signing_key: &p256::ecdsa::SigningKey,
) -> Result<EncodingKey, DpopError> {
    use p256::pkcs8::EncodePrivateKey;
    let der = signing_key
        .to_pkcs8_der()
        .map_err(|e| DpopError::InvalidProof(format!("failed to encode private key: {}", e)))?;
    Ok(EncodingKey::from_ec_der(der.as_bytes()))
}

pub(super) fn decoding_key_for_jwk(jwk: &Jwk) -> Result<DecodingKey, DpopError> {
    Ok(DecodingKey::from_ec_components(&jwk.x, &jwk.y)?)
}

pub(super) const DPOP_ALGORITHM: Algorithm = Algorithm::ES256;
