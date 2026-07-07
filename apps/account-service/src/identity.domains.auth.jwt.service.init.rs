use chrono::Duration;
use jsonwebtoken::DecodingKey;

use super::types::{JwtService, Signer};

impl JwtService {
    pub fn new_local(
        kid: &str,
        private_key_pem: &str,
        public_keys: Vec<(String, String)>,
        issuer: &str,
        audience: &str,
        refresh_token_expiry: Duration,
    ) -> Self {
        let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .expect("valid private key PEM at startup");
        let decoding_keys = public_keys
            .into_iter()
            .filter_map(|(k, pem)| {
                DecodingKey::from_rsa_pem(pem.as_bytes())
                    .ok()
                    .map(|dk| (k, dk))
            })
            .collect();
        Self {
            signer: Signer::Local { encoding_key },
            kid: kid.to_string(),
            decoding_keys,
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            access_token_expiry: Duration::minutes(15),
            refresh_token_expiry,
        }
    }
}
