use chrono::Duration;
use jsonwebtoken::DecodingKey;
use std::sync::Arc;

use super::{
    kms::ScalewayKmsSigner,
    types::{JwtService, Signer},
};

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

    pub fn new_kms(
        kid: &str,
        signer: ScalewayKmsSigner,
        public_keys: Vec<(String, String)>,
        issuer: &str,
        audience: &str,
        refresh_token_expiry: Duration,
    ) -> Self {
        let decoding_keys = public_keys
            .into_iter()
            .filter_map(|(key_id, pem)| {
                DecodingKey::from_rsa_pem(pem.as_bytes())
                    .ok()
                    .map(|key| (key_id, key))
            })
            .collect();
        Self {
            signer: Signer::Kms {
                signer: Arc::new(signer),
            },
            kid: kid.to_string(),
            decoding_keys,
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            access_token_expiry: Duration::minutes(15),
            refresh_token_expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use jsonwebtoken::{Algorithm, decode_header};
    use openssl::{pkey::PKey, rsa::Rsa};
    use uuid::Uuid;

    use super::JwtService;
    use crate::domains::auth::jwt::TokenPairIssueRequest;

    fn test_service(audience: &str) -> JwtService {
        let key = PKey::from_rsa(Rsa::generate(2048).expect("generate RSA key"))
            .expect("create private key");
        let private_key_pem =
            String::from_utf8(key.private_key_to_pem_pkcs8().expect("encode private key"))
                .expect("private key PEM");
        let public_key_pem = String::from_utf8(key.public_key_to_pem().expect("encode public key"))
            .expect("public key PEM");
        JwtService::new_local(
            "test-key",
            &private_key_pem,
            vec![("test-key".to_string(), public_key_pem)],
            "https://identity.example",
            audience,
            Duration::days(30),
        )
    }

    #[tokio::test]
    async fn aws_lc_provider_preserves_ps256_pem_compatibility() {
        let service = test_service("nvbes-api");

        let tokens = service
            .generate_token_pair(Uuid::new_v4(), None, "openid")
            .await
            .expect("sign access token");
        let header = decode_header(&tokens.access_token).expect("decode access token header");
        let claims = service
            .decode_token(&tokens.access_token, "access")
            .expect("verify access token");

        assert_eq!(header.alg, Algorithm::PS256);
        assert_eq!(header.kid.as_deref(), Some("test-key"));
        assert_eq!(claims.iss, "https://identity.example");
        assert_eq!(claims.aud, "nvbes-api");
    }

    #[tokio::test]
    async fn token_pair_uses_the_explicit_resource_audience_only_for_access() {
        let service = test_service("nvbes-identity-service");
        let mut request =
            TokenPairIssueRequest::new(Uuid::new_v4(), "nvbes-cloud-service", "drive.files.read");
        request.acr = Some("aal1");
        request.amr = Some(vec!["pwd".to_string()]);

        let tokens = service
            .issue_token_pair(request)
            .await
            .expect("sign audience-bound pair");
        let access_claims = service
            .decode_token(&tokens.access_token, "access")
            .expect("verify access token");
        let refresh_claims = service
            .decode_token(&tokens.refresh_token, "refresh")
            .expect("verify refresh token");

        assert_eq!(access_claims.aud, "nvbes-cloud-service");
        assert_eq!(refresh_claims.aud, "nvbes-identity-service");
    }
}
