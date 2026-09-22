//! Shared HTTP + JWT fixtures for Account service integration tests.

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use openssl::{pkey::PKey, rsa::Rsa};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::OnceLock;
use uuid::Uuid;

use crate::{app::AccountState, auth::TokenVerifier, config::AccountConfig};

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

pub(crate) fn account_config() -> AccountConfig {
    AccountConfig {
        environment: "test".into(),
        database_url: "postgres://unused".into(),
        database_max_connections: 1,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        token_issuer: "http://identity.local".into(),
        token_audience: "nvbes-account".into(),
        token_key_id: "identity-key-1".into(),
        token_public_key_pem: test_keys().public_pem.clone(),
        metrics_token: "development-account-metrics-token-value".into(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.1,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    }
}

pub(crate) fn state_with_pool(pool: PgPool) -> AccountState {
    AccountState::new(account_config(), pool).expect("account state")
}

pub(crate) fn access_token(principal_id: Uuid, scopes: &str, step_up: bool) -> String {
    let now = jsonwebtoken::get_current_timestamp();
    let mut amr = vec!["pwd".to_owned()];
    if step_up {
        amr.push("totp".into());
    }
    let claims = Claims {
        sub: principal_id.to_string(),
        token_type: "access".into(),
        scope: scopes.into(),
        amr,
        iss: "http://identity.local".into(),
        aud: "nvbes-account".into(),
        exp: now + 600,
        iat: now,
        nbf: now,
        jti: Uuid::new_v4().to_string(),
        sid: Uuid::new_v4().to_string(),
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("identity-key-1".into());
    header.typ = Some("at+jwt".into());
    let key = EncodingKey::from_rsa_pem(test_keys().private_pem.as_bytes()).expect("encoding");
    encode(&header, &claims, &key).unwrap()
}

#[allow(dead_code)]
pub(crate) fn verifier() -> TokenVerifier {
    TokenVerifier::new(&account_config()).unwrap()
}
