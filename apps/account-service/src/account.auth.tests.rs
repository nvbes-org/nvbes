use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use openssl::{pkey::PKey, rsa::Rsa};
use serde::Serialize;
use std::sync::OnceLock;
use uuid::Uuid;

use crate::config::AccountConfig;
use crate::error::AccountError;

use super::{Principal, TokenVerifier};

struct TestKeys {
    private_pem: String,
    public_pem: String,
}

fn test_keys() -> &'static TestKeys {
    static KEYS: OnceLock<TestKeys> = OnceLock::new();
    KEYS.get_or_init(|| {
        let private = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
        TestKeys {
            private_pem: String::from_utf8(private.private_key_to_pem_pkcs8().expect("priv pem"))
                .expect("utf8"),
            public_pem: String::from_utf8(private.public_key_to_pem().expect("pub pem"))
                .expect("utf8"),
        }
    })
}

#[derive(Serialize)]
struct Claims {
    sub: String,
    token_type: String,
    scope: String,
    amr: Vec<String>,
    iss: String,
    aud: String,
    exp: u64,
    iat: u64,
    nbf: u64,
    jti: String,
    sid: String,
}

fn config() -> AccountConfig {
    AccountConfig {
        environment: "test".into(),
        database_url: "postgres://localhost/account".into(),
        database_max_connections: 1,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        token_issuer: "http://identity.local".into(),
        token_audience: "nvbes-account-service".into(),
        token_key_id: "identity-key-1".into(),
        token_public_key_pem: test_keys().public_pem.clone(),
        metrics_token: "development-account-metrics-token-value".into(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.1,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    }
}

fn encoding_key() -> EncodingKey {
    EncodingKey::from_rsa_pem(test_keys().private_pem.as_bytes()).expect("encoding key")
}

fn encode_access(mut claims: Claims) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("identity-key-1".into());
    header.typ = Some("at+jwt".into());
    if claims.jti.is_empty() {
        claims.jti = Uuid::new_v4().to_string();
    }
    if claims.sid.is_empty() {
        claims.sid = Uuid::new_v4().to_string();
    }
    encode(&header, &claims, &encoding_key()).unwrap()
}

fn valid_claims(now: u64) -> Claims {
    Claims {
        sub: Uuid::new_v4().to_string(),
        token_type: "access".into(),
        scope: "account:read account:write".into(),
        amr: vec!["pwd".into(), "totp".into()],
        iss: "http://identity.local".into(),
        aud: "nvbes-account-service".into(),
        exp: now + 600,
        iat: now,
        nbf: now,
        jti: Uuid::new_v4().to_string(),
        sid: Uuid::new_v4().to_string(),
    }
}

#[test]
fn destructive_actions_require_a_step_up_method() {
    let password_only = Principal {
        id: Uuid::new_v4(),
        scopes: vec!["account:close".into()],
        authentication_methods: vec!["pwd".into()],
    };
    assert!(password_only.require("account:close").is_ok());
    assert!(password_only.require("account:read").is_err());
    assert!(password_only.require_step_up().is_err());

    let stepped_up = Principal {
        authentication_methods: vec!["pwd".into(), "webauthn".into()],
        ..password_only
    };
    assert!(stepped_up.require_step_up().is_ok());
}

#[test]
fn token_verifier_accepts_valid_access_token() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let now = jsonwebtoken::get_current_timestamp();
    let token = encode_access(valid_claims(now));
    let principal = verifier.verify(&token).unwrap();
    assert!(principal.require("account:read").is_ok());
    assert!(principal.require_step_up().is_ok());
}

#[test]
fn token_verifier_rejects_wrong_type_kid_and_claims() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let now = jsonwebtoken::get_current_timestamp();

    assert!(matches!(
        verifier.verify("not-a-jwt"),
        Err(AccountError::Unauthorized)
    ));

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("wrong-kid".into());
    header.typ = Some("at+jwt".into());
    let bad_kid = encode(&header, &valid_claims(now), &encoding_key()).unwrap();
    assert!(verifier.verify(&bad_kid).is_err());

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("identity-key-1".into());
    header.typ = Some("JWT".into());
    let bad_typ = encode(&header, &valid_claims(now), &encoding_key()).unwrap();
    assert!(verifier.verify(&bad_typ).is_err());

    let mut claims = valid_claims(now);
    claims.token_type = "refresh".into();
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.exp = claims.iat;
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.nbf = claims.iat + 10;
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.jti = "not-a-uuid".into();
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.sid = "not-a-uuid".into();
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.iss = "http://other.local".into();
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.aud = "other-audience".into();
    assert!(verifier.verify(&encode_access(claims)).is_err());

    let mut claims = valid_claims(now);
    claims.sub = "not-a-uuid".into();
    assert!(verifier.verify(&encode_access(claims)).is_err());
}

#[test]
fn token_verifier_rejects_invalid_pem() {
    let mut bad = config();
    bad.token_public_key_pem = "not-a-pem".into();
    assert!(TokenVerifier::new(&bad).is_err());
}

#[tokio::test]
async fn principal_extractor_requires_bearer_token() {
    use axum::extract::FromRequestParts;
    use axum::http::{HeaderMap, HeaderValue, Request};

    let state = crate::app::AccountState::new(
        config(),
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/account")
            .expect("lazy pool"),
    )
    .expect("state");

    let mut missing = Request::builder().uri("/").body(()).unwrap().into_parts().0;
    assert!(matches!(
        Principal::from_request_parts(&mut missing, &state).await,
        Err(AccountError::Unauthorized)
    ));

    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Basic not-bearer"),
    );
    let mut request = Request::builder().uri("/").body(()).unwrap();
    *request.headers_mut() = headers;
    let mut non_bearer = request.into_parts().0;
    assert!(matches!(
        Principal::from_request_parts(&mut non_bearer, &state).await,
        Err(AccountError::Unauthorized)
    ));

    let token = encode_access(valid_claims(jsonwebtoken::get_current_timestamp()));
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    let mut request = Request::builder().uri("/").body(()).unwrap();
    *request.headers_mut() = headers;
    let mut authorized = request.into_parts().0;
    let principal = Principal::from_request_parts(&mut authorized, &state)
        .await
        .expect("authorized");
    assert!(principal.require("account:read").is_ok());
}
