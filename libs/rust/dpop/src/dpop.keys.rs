use base64::Engine;
use p256::SecretKey;
use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    #[serde(rename = "kty")]
    pub kty: String,
    #[serde(rename = "crv")]
    pub crv: String,
    #[serde(rename = "x")]
    pub x: String,
    #[serde(rename = "y")]
    pub y: String,
}

impl Jwk {
    pub fn from_verifying_key(key: &VerifyingKey) -> Self {
        let encoded = key.to_encoded_point(false);
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        Self {
            kty: "EC".to_string(),
            crv: "P-256".to_string(),
            x: b64.encode(encoded.x().expect("x coordinate")),
            y: b64.encode(encoded.y().expect("y coordinate")),
        }
    }

    pub fn to_verifying_key(&self) -> Result<VerifyingKey, DpopKeyError> {
        if self.kty != "EC" || self.crv != "P-256" {
            return Err(DpopKeyError::UnsupportedKey(
                self.kty.clone(),
                self.crv.clone(),
            ));
        }
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let x = b64.decode(&self.x).map_err(|_| DpopKeyError::InvalidJwk)?;
        let y = b64.decode(&self.y).map_err(|_| DpopKeyError::InvalidJwk)?;
        let mut raw = vec![0x04];
        raw.extend_from_slice(&x);
        raw.extend_from_slice(&y);
        VerifyingKey::from_sec1_bytes(&raw).map_err(|_| DpopKeyError::InvalidPublicKey)
    }
}

#[derive(Debug)]
pub struct DpopKeyPair {
    pub secret_key: SecretKey,
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub jwk: Jwk,
    pub jkt: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DpopKeyError {
    #[error("unsupported key: kty={0}, crv={1}")]
    UnsupportedKey(String, String),
    #[error("invalid JWK format")]
    InvalidJwk,
    #[error("invalid public key bytes")]
    InvalidPublicKey,
}

pub fn generate_key_pair() -> DpopKeyPair {
    let secret_key = SecretKey::random(&mut rand_core::OsRng);
    let signing_key = SigningKey::from(&secret_key);
    let verifying_key = VerifyingKey::from(&signing_key);
    let jwk = Jwk::from_verifying_key(&verifying_key);
    let jkt = jwk_thumbprint(&jwk);
    DpopKeyPair {
        secret_key,
        signing_key,
        verifying_key,
        jwk,
        jkt,
    }
}

pub fn jwk_thumbprint(jwk: &Jwk) -> String {
    let canonical = serde_json::json!({
        "crv": jwk.crv,
        "kty": jwk.kty,
        "x": jwk.x,
        "y": jwk.y,
    });
    let json = serde_json::to_string(&canonical).expect("JWK serialization never fails");
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_verify_jwk_roundtrip() {
        let pair = generate_key_pair();
        let recovered = pair
            .jwk
            .to_verifying_key()
            .expect("roundtrip should succeed");
        assert_eq!(
            pair.verifying_key.to_encoded_point(false).as_bytes(),
            recovered.to_encoded_point(false).as_bytes()
        );
    }

    #[test]
    fn jwk_thumbprint_is_stable() {
        let jwk = Jwk {
            kty: "EC".into(),
            crv: "P-256".into(),
            x: "some-x-value".into(),
            y: "some-y-value".into(),
        };
        let tp1 = jwk_thumbprint(&jwk);
        let tp2 = jwk_thumbprint(&jwk);
        assert_eq!(tp1, tp2);
        assert!(!tp1.is_empty());
    }
}
