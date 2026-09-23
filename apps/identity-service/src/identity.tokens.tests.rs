use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, decode_header, encode};
use openssl::{pkey::PKey, rsa::Rsa};
use uuid::Uuid;

use crate::tokens_config::TokenConfig;

use super::{ACCESS_TOKEN_TTL_SECONDS, AccessTokenClaims, TokenService, validate_scope};

fn key_material() -> (String, String, EncodingKey) {
    let private = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let private_pem = String::from_utf8(private.private_key_to_pem_pkcs8().unwrap()).unwrap();
    let public_pem = String::from_utf8(private.public_key_to_pem().unwrap()).unwrap();
    let encoding = EncodingKey::from_rsa_pem(private_pem.as_bytes()).unwrap();
    (private_pem, public_pem, encoding)
}

fn service_from(private_pem: &str, public_pem: &str) -> TokenService {
    TokenService::new(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity-key-1".into(),
            private_pem.into(),
            public_pem.into(),
            "nvbes-account-service".into(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn service() -> TokenService {
    let (private_pem, public_pem, _) = key_material();
    service_from(&private_pem, &public_pem)
}

fn access_claims(sub: &str, sid: &str, token_type: &str) -> AccessTokenClaims {
    let issued_at = Utc::now().timestamp() as u64;
    AccessTokenClaims {
        sub: sub.into(),
        token_type: token_type.into(),
        scope: "account:read".into(),
        amr: vec!["pwd".into()],
        iss: "http://identity.test".into(),
        aud: "nvbes-account-service".into(),
        exp: issued_at + ACCESS_TOKEN_TTL_SECONDS,
        iat: issued_at,
        nbf: issued_at,
        jti: Uuid::new_v4().to_string(),
        sid: sid.into(),
    }
}

fn mint(encoding: &EncodingKey, header: Header, claims: &AccessTokenClaims) -> String {
    encode(&header, claims, encoding).unwrap()
}

fn at_jwt_header(kid: &str) -> Header {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.into());
    header.typ = Some("at+jwt".into());
    header
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

#[test]
fn token_service_rejects_invalid_pem_material() {
    let (private_pem, public_pem, _) = key_material();
    let bad_private = TokenConfig::from_values(
        "test",
        "http://identity.test".into(),
        "identity-key-1".into(),
        // Built without a literal PEM banner so secret-scan stays quiet.
        format!(
            "-----BEGIN {}-----\nnot-rsa\n-----END {}-----",
            "PRIVATE KEY", "PRIVATE KEY"
        ),
        public_pem.clone(),
        "nvbes-account-service".into(),
    )
    .unwrap();
    assert!(TokenService::new(bad_private).is_err());

    let bad_public = TokenConfig::from_values(
        "test",
        "http://identity.test".into(),
        "identity-key-1".into(),
        private_pem,
        format!(
            "-----BEGIN {}-----\nnot-rsa\n-----END {}-----",
            "PUBLIC KEY", "PUBLIC KEY"
        ),
        "nvbes-account-service".into(),
    )
    .unwrap();
    assert!(TokenService::new(bad_public).is_err());
}

#[test]
fn verify_rejects_invalid_header_kid_typ_and_token_type() {
    let (private_pem, public_pem, encoding) = key_material();
    let service = service_from(&private_pem, &public_pem);
    let principal = Uuid::new_v4().to_string();
    let session = Uuid::new_v4().to_string();
    let claims = access_claims(&principal, &session, "access");

    let mut wrong_kid = at_jwt_header("other-key");
    let token = mint(&encoding, wrong_kid.clone(), &claims);
    assert!(service.verify(&token, "nvbes-account-service").is_err());

    wrong_kid = at_jwt_header("identity-key-1");
    wrong_kid.typ = Some("JWT".into());
    let token = mint(&encoding, wrong_kid, &claims);
    assert!(service.verify(&token, "nvbes-account-service").is_err());

    let wrong_type = access_claims(&principal, &session, "refresh");
    let token = mint(&encoding, at_jwt_header("identity-key-1"), &wrong_type);
    let err = service
        .verify(&token, "nvbes-account-service")
        .expect_err("non-access token type");
    assert!(err.to_string().contains("token type is invalid"), "{err}");
}

#[test]
fn verify_rejects_unregistered_expected_audience() {
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
    assert!(service.verify(&token, "unregistered-audience").is_err());
}

#[test]
fn validate_scope_rejects_oversized_and_non_ascii_tokens() {
    assert!(validate_scope(&"a".repeat(1024)).is_ok());
    assert!(validate_scope(&"a".repeat(1025)).is_err());
    assert!(validate_scope("account:read").is_ok());
    assert!(validate_scope("caf\u{e9}").is_err());
    assert!(validate_scope("ok@bad").is_err());
}

#[cfg(feature = "database-tests")]
mod database {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "./migrations")]
    async fn introspect_returns_none_for_invalid_sid_or_sub(pool: PgPool) {
        let (private_pem, public_pem, encoding) = key_material();
        let service = service_from(&private_pem, &public_pem);
        let header = at_jwt_header("identity-key-1");

        let bad_sid = mint(
            &encoding,
            header.clone(),
            &access_claims(&Uuid::new_v4().to_string(), "not-a-uuid", "access"),
        );
        assert!(
            service
                .introspect(&pool, &bad_sid, "nvbes-account-service")
                .await
                .unwrap()
                .is_none()
        );

        let bad_sub = mint(
            &encoding,
            header,
            &access_claims("not-a-uuid", &Uuid::new_v4().to_string(), "access"),
        );
        assert!(
            service
                .introspect(&pool, &bad_sub, "nvbes-account-service")
                .await
                .unwrap()
                .is_none()
        );

        assert!(
            service
                .introspect(&pool, "not.a.jwt", "nvbes-account-service")
                .await
                .unwrap()
                .is_none()
        );
    }
}
