use chrono::Utc;
use jsonwebtoken::{Header, encode};

use crate::keys::DpopKeyPair;

use super::{
    DpopError, DpopProofClaims,
    jwk::{DPOP_ALGORITHM, jwk_to_jwt_jwk, p256_to_jwt_signing_key},
};

pub fn create_dpop_proof(
    key_pair: &DpopKeyPair,
    htm: &str,
    htu: &str,
    access_token: Option<&str>,
    nonce: Option<&str>,
) -> Result<String, DpopError> {
    let jti = uuid::Uuid::new_v4().to_string();
    let iat = Utc::now().timestamp();

    let ath = access_token.map(super::verify::compute_ath);

    let claims = DpopProofClaims {
        jti,
        htm: htm.to_uppercase(),
        htu: htu.to_string(),
        iat,
        ath,
        nonce: nonce.map(|n| n.to_string()),
    };

    let mut header = Header::new(DPOP_ALGORITHM);
    header.typ = Some("dpop+jwt".to_string());
    header.jwk = Some(jwk_to_jwt_jwk(&key_pair.jwk));

    let signing_key = p256_to_jwt_signing_key(&key_pair.signing_key)?;
    Ok(encode(&header, &claims, &signing_key)?)
}
