use crate::{
    tokens_claims::{JsonWebKey, JsonWebKeySet},
    tokens_config::TokenConfig,
    tokens_error::TokenError,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, traits::PublicKeyParts};
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) struct KeyRing {
    active_kid: String,
    signing: EncodingKey,
    verifying: BTreeMap<String, VerificationKey>,
}

struct VerificationKey {
    key: DecodingKey,
    jwk: JsonWebKey,
    accept_until: Option<u64>,
}

impl KeyRing {
    pub fn new(config: &TokenConfig) -> Result<Self, TokenError> {
        let signing = EncodingKey::from_rsa_pem(config.private_key_pem.as_bytes())?;
        let active = verification_key(&config.key_id, &config.public_key_pem, None)?;
        // Fail startup if the PEM pair does not match, before issuing any tokens.
        let probe = encode(
            &Header::new(Algorithm::RS256),
            &serde_json::json!({"exp":u64::MAX}),
            &signing,
        )?;
        decode::<serde_json::Value>(&probe, &active.key, &Validation::new(Algorithm::RS256))?;
        let mut verifying = BTreeMap::from([(config.key_id.clone(), active)]);
        for key in &config.verification_keys {
            verifying.insert(
                key.kid.clone(),
                verification_key(&key.kid, &key.public_key_pem, Some(key.accept_until))?,
            );
        }
        Ok(Self {
            active_kid: config.key_id.clone(),
            signing,
            verifying,
        })
    }

    pub fn sign<T: Serialize>(&self, typ: &str, claims: &T) -> Result<String, TokenError> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.active_kid.clone());
        header.typ = Some(typ.into());
        Ok(encode(&header, claims, &self.signing)?)
    }

    pub fn decoding_key(&self, kid: &str, now: u64) -> Result<&DecodingKey, TokenError> {
        self.verifying
            .get(kid)
            .filter(|key| key.accept_until.is_none_or(|until| until > now))
            .map(|key| &key.key)
            .ok_or(TokenError::InvalidToken)
    }

    pub fn jwks(&self, now: u64) -> JsonWebKeySet {
        JsonWebKeySet {
            keys: self
                .verifying
                .values()
                .filter(|key| key.accept_until.is_none_or(|until| until > now))
                .map(|key| key.jwk.clone())
                .collect(),
        }
    }
}

fn verification_key(
    kid: &str,
    pem: &str,
    accept_until: Option<u64>,
) -> Result<VerificationKey, TokenError> {
    let public = RsaPublicKey::from_public_key_pem(pem)
        .map_err(|_| TokenError::Configuration("RSA public key"))?;
    if !(2048..=4096).contains(&public.n().bits()) || public.e() != &rsa::BigUint::from(65_537_u32)
    {
        return Err(TokenError::Configuration(
            "RSA key must be 2048-4096 bits with exponent 65537",
        ));
    }
    Ok(VerificationKey {
        key: DecodingKey::from_rsa_pem(pem.as_bytes())?,
        accept_until,
        jwk: JsonWebKey {
            kid: kid.into(),
            kty: "RSA",
            usage: "sig",
            alg: "RS256",
            n: URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()),
            e: URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()),
        },
    })
}
