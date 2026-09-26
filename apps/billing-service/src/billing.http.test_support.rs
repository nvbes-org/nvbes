//! Shared HTTP router and JWT fixtures for billing integration tests.

use std::sync::OnceLock;

use axum::Router;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use openssl::{pkey::PKey, rsa::Rsa};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::{BillingState, create_router},
    auth::TokenVerifier,
    config::BillingConfig,
    database::test_support::test_config,
    metrics::install,
};

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
struct AccessClaims {
    sub: String,
    scope: Option<String>,
    amr: Option<Vec<String>>,
    exp: u64,
    iat: u64,
}

pub fn jwt_config() -> BillingConfig {
    let mut config = test_config();
    config.identity_public_key_pem = Some(test_keys().public_pem.clone());
    config
}

pub fn access_token(principal_id: Uuid, scopes: &str) -> String {
    let now = jsonwebtoken::get_current_timestamp();
    let claims = AccessClaims {
        sub: principal_id.to_string(),
        scope: Some(scopes.into()),
        amr: Some(vec!["pwd".into()]),
        exp: now + 600,
        iat: now,
    };
    let header = Header::new(Algorithm::RS256);
    let key = EncodingKey::from_rsa_pem(test_keys().private_pem.as_bytes()).expect("encoding");
    encode(&header, &claims, &key).expect("jwt")
}

pub fn bearer(principal_id: Uuid) -> String {
    format!("Bearer {}", principal_id)
}

pub fn bearer_jwt(principal_id: Uuid, scopes: &str) -> String {
    format!("Bearer {}", access_token(principal_id, scopes))
}

pub fn test_router(pool: PgPool) -> Router {
    test_router_with_config(pool, test_config())
}

pub fn test_router_with_config(pool: PgPool, config: BillingConfig) -> Router {
    let tokens = TokenVerifier::new(&config).expect("verifier");
    let state = BillingState {
        db: pool,
        config,
        metrics: install(),
        tokens,
        email_client: None,
    };
    create_router(state)
}
