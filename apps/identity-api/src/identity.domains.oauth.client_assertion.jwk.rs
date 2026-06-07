use crate::http::error::AppError;
use jsonwebtoken::{Algorithm, DecodingKey};

use super::ClientAssertionJwk;

pub(crate) fn supported_algorithm(algorithm: Algorithm) -> Result<Algorithm, AppError> {
    match algorithm {
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 | Algorithm::ES256 => Ok(algorithm),
        _ => Err(AppError::unauthorized(
            "invalid_client",
            "Unsupported client_assertion signing algorithm.",
        )),
    }
}

pub(crate) fn validate_jwk_header_consistency(
    jwk: &ClientAssertionJwk,
    header_kid: Option<&str>,
    algorithm: Algorithm,
) -> Result<(), AppError> {
    if let (Some(header_kid), Some(jwk_kid)) = (header_kid, jwk.kid.as_deref()) {
        if header_kid != jwk_kid {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The client_assertion kid does not match the registered client key.",
            ));
        }
    }

    if let Some(jwk_alg) = jwk.alg.as_deref() {
        if algorithm_from_name(jwk_alg) != Some(algorithm) {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The client_assertion alg does not match the registered client key.",
            ));
        }
    }

    Ok(())
}

pub(crate) fn decoding_key_for_jwk(
    jwk: &ClientAssertionJwk,
    algorithm: Algorithm,
) -> Result<DecodingKey, AppError> {
    match (jwk.kty.as_str(), algorithm) {
        ("RSA", Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512) => {
            let n = jwk.n.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion RSA key is invalid.")
            })?;
            let e = jwk.e.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion RSA key is invalid.")
            })?;
            DecodingKey::from_rsa_components(n, e).map_err(|_| {
                AppError::unauthorized("invalid_client", "The client_assertion RSA key is invalid.")
            })
        }
        ("EC", Algorithm::ES256) => {
            if jwk.crv.as_deref() != Some("P-256") {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "Only P-256 client_assertion EC keys are supported.",
                ));
            }
            let x = jwk.x.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion EC key is invalid.")
            })?;
            let y = jwk.y.as_deref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "The client_assertion EC key is invalid.")
            })?;
            DecodingKey::from_ec_components(x, y).map_err(|_| {
                AppError::unauthorized("invalid_client", "The client_assertion EC key is invalid.")
            })
        }
        _ => Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion key type is not compatible with the signing algorithm.",
        )),
    }
}

pub(crate) fn algorithm_from_name(value: &str) -> Option<Algorithm> {
    match value {
        "RS256" => Some(Algorithm::RS256),
        "RS384" => Some(Algorithm::RS384),
        "RS512" => Some(Algorithm::RS512),
        "ES256" => Some(Algorithm::ES256),
        _ => None,
    }
}

pub fn is_supported_client_assertion_public_jwk(value: &serde_json::Value) -> bool {
    let Ok(jwk) = serde_json::from_value::<ClientAssertionJwk>(value.clone()) else {
        return false;
    };
    let Some(algorithm) = jwk.alg.as_deref().and_then(algorithm_from_name) else {
        return false;
    };

    supported_algorithm(algorithm).is_ok() && decoding_key_for_jwk(&jwk, algorithm).is_ok()
}
