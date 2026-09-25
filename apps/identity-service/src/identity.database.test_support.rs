//! Shared fixtures for Identity `database-tests` HTTP and OAuth suites.

#![allow(dead_code)]

use std::sync::{Mutex, MutexGuard, OnceLock};

use chrono::{Duration, Utc};
use openssl::{pkey::PKey, rsa::Rsa};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{app::IdentityState, auth::hash_token, config::IdentityConfig};

/// Shared lock for any Identity test that mutates process environment.
/// Token issuance reads `NVBES_IDENTITY_TOKEN_*` at request time, so all
/// env writers must serialize through this lock to avoid cross-test flakes.
pub fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub struct TokenEnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(String, Option<String>)>,
}

impl TokenEnvGuard {
    pub fn install_test_keys() -> Self {
        let lock = test_env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let private = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
        let private_pem =
            String::from_utf8(private.private_key_to_pem_pkcs8().expect("priv")).expect("utf8");
        let public_pem =
            String::from_utf8(private.public_key_to_pem().expect("pub")).expect("utf8");
        let pairs = [
            ("NVBES_IDENTITY_TOKEN_KEY_ID", "identity-key-db-test"),
            ("NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM", private_pem.as_str()),
            ("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM", public_pem.as_str()),
            ("NVBES_IDENTITY_TOKEN_AUDIENCES", "account,default"),
            ("NVBES_IDENTITY_TOKEN_ISSUER", "http://identity.test"),
        ];
        let mut saved = Vec::with_capacity(pairs.len());
        for (name, value) in pairs {
            saved.push((name.to_string(), std::env::var(name).ok()));
            // SAFETY: serialized by `test_env_lock` for test-only env mutation.
            unsafe { std::env::set_var(name, value) };
        }
        Self { _lock: lock, saved }
    }
}

impl Drop for TokenEnvGuard {
    fn drop(&mut self) {
        for (name, previous) in &self.saved {
            match previous {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

/// Clears token key env vars while holding the shared test env lock.
pub struct WithoutTokenKeysGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(String, Option<String>)>,
}

impl WithoutTokenKeysGuard {
    pub fn install() -> Self {
        let lock = test_env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let names = [
            "NVBES_IDENTITY_TOKEN_KEY_ID",
            "NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM",
            "NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM",
            "NVBES_IDENTITY_TOKEN_AUDIENCES",
        ];
        let mut saved = Vec::with_capacity(names.len());
        for name in names {
            saved.push((name.to_string(), std::env::var(name).ok()));
            // SAFETY: serialized by `test_env_lock` for test-only env mutation.
            unsafe { std::env::remove_var(name) };
        }
        Self { _lock: lock, saved }
    }
}

impl Drop for WithoutTokenKeysGuard {
    fn drop(&mut self) {
        for (name, previous) in &self.saved {
            match previous {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

pub fn identity_state(pool: PgPool) -> IdentityState {
    IdentityState::new(
        IdentityConfig {
            environment: "test".into(),
            sentry_dsn: None,
            sentry_traces_sample_rate: 0.1,
            otlp_endpoint: None,
            otlp_authorization_header: None,
            metrics_token: "identity-metrics-test-token-32-characters".into(),
            database_url: "postgres://unused".into(),
            database_max_connections: 2,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            mfa_encryption_key: [2; 32],
            mfa_key_version: 1,
            mfa_previous_encryption_key: None,
            mfa_previous_key_version: None,
            token_issuer: "http://identity.test".into(),
            platform_operator_principals: Default::default(),
            public_signup_enabled: true,
            login_url: String::new(),
            session_cookie_secure: false,
        },
        pool,
    )
}

pub async fn seed_principal_and_session(pool: &PgPool) -> (Uuid, Uuid, String) {
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let session_secret = format!("session-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO identity_principals (id, kind, status) VALUES ($1, 'human', 'active')",
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("principal");
    sqlx::query(
        "INSERT INTO identity_sessions (id, principal_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(session_id)
    .bind(principal_id)
    .bind(hash_token(&session_secret))
    .bind(Utc::now() + Duration::hours(2))
    .execute(pool)
    .await
    .expect("session");
    (principal_id, session_id, session_secret)
}

pub async fn seed_confidential_oauth_client(
    pool: &PgPool,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) {
    sqlx::query(
        "INSERT INTO identity_oauth_clients (id, client_id, client_secret_hash, name, redirect_uris, scopes, is_confidential)
         VALUES ($1, $2, $3, $4, $5, $6, true)
         ON CONFLICT (client_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(client_id)
    .bind(hash_token(client_secret))
    .bind("Database test client")
    .bind(vec![redirect_uri.to_string()])
    .bind(vec![
        "openid".to_string(),
        "account:read".to_string(),
        "account:write".to_string(),
    ])
    .execute(pool)
    .await
    .expect("oauth client");
}

pub async fn seed_public_oauth_client(pool: &PgPool, client_id: &str, redirect_uri: &str) {
    sqlx::query(
        "INSERT INTO identity_oauth_clients (id, client_id, client_secret_hash, name, redirect_uris, scopes, is_confidential)
         VALUES ($1, $2, $3, $4, $5, $6, false)
         ON CONFLICT (client_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(client_id)
    .bind(hash_token("unused-public-client-placeholder"))
    .bind("Database test public client")
    .bind(vec![redirect_uri.to_string()])
    .bind(vec!["openid".to_string(), "profile".to_string()])
    .execute(pool)
    .await
    .expect("public oauth client");
}
