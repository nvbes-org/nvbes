use jsonwebtoken::{Algorithm, decode_header};
use uuid::Uuid;

use crate::{
    test_keys::{TEST_RSA_PUBLIC_KEY_PEM, test_rsa_private_key_pem},
    tokens_config::TokenConfig,
};

use super::{ACCESS_TOKEN_TTL_SECONDS, TokenService};

fn service() -> TokenService {
    TokenService::new(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity-key-1".into(),
            test_rsa_private_key_pem(),
            TEST_RSA_PUBLIC_KEY_PEM.to_string(),
            "nvbes-account-service".into(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn access_token_is_rs256_audience_bound_and_short_lived() {
    let service = service();
    let token = service
        .issue(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "nvbes-account-service",
            "account:read",
            vec!["pwd".into()],
        )
        .unwrap();
    let header = decode_header(&token).unwrap();
    assert_eq!(header.alg, Algorithm::RS256);
    assert_eq!(header.kid.as_deref(), Some("identity-key-1"));
    assert_eq!(header.typ.as_deref(), Some("at+jwt"));
    let claims = service.verify(&token, "nvbes-account-service").unwrap();
    assert_eq!(claims.exp - claims.iat, ACCESS_TOKEN_TTL_SECONDS);
    assert!(service.verify(&token, "nvbes-billing-service").is_err());
    assert_eq!(service.jwks().keys[0].alg, "RS256");
}

#[test]
fn issue_rejects_unregistered_audience_and_malformed_scope() {
    let service = service();
    let principal = Uuid::new_v4();
    let session = Uuid::new_v4();
    assert!(
        service
            .issue(
                principal,
                session,
                "unknown",
                "account:read",
                vec!["pwd".into()]
            )
            .is_err()
    );
    assert!(
        service
            .issue(
                principal,
                session,
                "nvbes-account-service",
                "account:read  admin",
                vec!["pwd".into()]
            )
            .is_err()
    );
}
